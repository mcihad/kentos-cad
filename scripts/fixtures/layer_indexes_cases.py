"""Writes the shared cases of a layer's indexes and keys (docs/adr/0758) without KentOS code:

- fixtures/commands/v1/cad.layers.indexes.json: the indexes themselves (§1, §2);
- fixtures/commands/v1/cad.entities.set.indexes.json and cad.polygon.create.indexes.json: what a unique index (the
  layer's key) does to the records' writes (§3);
- fixtures/commands/v1/cad.fields.write.indexes.json: a batch of writes a key refuses some of (§3);
- fixtures/commands/v1/cad.forms.validate.indexes.json: a form over the layer says the key on its first shown field (§3);
- fixtures/commands/v1/cad.records.duplicate.indexes.json: a copy leaves the layer's unique fields and keys empty (§3).

    python3 scripts/fixtures/layer_indexes_cases.py           # writes the files
    python3 scripts/fixtures/layer_indexes_cases.py --check   # compares them with the ones on disk

The rule is the ADR's, written here again: an index over 1–32 of the layer's fields (by their keys, each once, none
holding bytes), its name once in the layer folded the Turkish way, `nullsEqual` only with `unique`, no two over the same
fields in the same order, at most 64. A unique index compares the canonical values of its fields; an empty value (empty or
blank text) is none, and a set with one is not compared unless `nullsEqual`. A write is asked only when it changes a field
of the index (a new record always); its refusal points at the first field of the index it changed. A new unique index, or
one stricter than before, that the records already break is refused with their ids. The form's twin is forms_cases.py; a
field's values are the layer fields' twin's (layer_field_cases.py).
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402
import layer_field_cases as lf  # noqa: E402

OUT = ROOT / "fixtures/commands/v1"
ANY = "$any"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
E0, N0 = 487000, 4420000
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
JS_SPACE = "\t\n\x0b\x0c\r \xa0              " \
           "  　﻿"

FIELDS = [
    {"name": "Ada", "kind": "integer"},
    {"name": "Parsel", "kind": "integer"},
    {"name": "Mahalle", "kind": "text"},
    {"id": "bina-no", "name": "Bina no", "kind": "text"},
    {"name": "Belge", "kind": "binary"},
    {"name": "Alan", "kind": "decimal", "scale": 2},
]
KEY = {"name": "Ada ve parsel", "fields": ["Ada", "Parsel"], "unique": True}
MAH = {"name": "Mahalle", "fields": ["Mahalle"]}
BINA = {"name": "Bina", "fields": ["bina-no"], "unique": True, "nullsEqual": True}

RECORDS = {
    1: {"Ada": "101", "Parsel": "5", "Mahalle": "Merkez", "Bina no": "B1"},
    2: {"Ada": "101", "Parsel": "6", "Mahalle": "Merkez", "Bina no": "B2"},
    3: {"Ada": "102", "Parsel": "5", "Bina no": "B3"},
    4: {"Ada": "103", "Bina no": "B4"},
    5: {"Ada": "103", "Bina no": "B5"},
    6: {"Alan": "250.00"},
}


def key(f):
    return f.get("id") or f["name"]


def label(f):
    return f.get("alias") or f["name"]


def fold(text):
    """The contract's `fold`: I is ı's, İ is i's, any other letter its lowercase when that is one letter."""
    out = []
    for c in text:
        if c == "I":
            out.append("ı")
        elif c == "İ":
            out.append("i")
        else:
            low = c.lower()
            out.append(low if len(low) == 1 else c)
    return "".join(out)


def layer(i, name, fields=None, kind="layer", children=None, **more):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields:
        out["fields"] = copy.deepcopy(fields)
    out.update(more)
    return out


def layers(indexes=()):
    parsel = layer("parsel", "Parsel", FIELDS)
    if indexes:
        parsel["indexes"] = copy.deepcopy(list(indexes))
    return [parsel, layer("grup", "Mahalleler", kind="group", children=[layer("alt", "Sokak")]),
            layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                                                      "maxZoom": 19})]


def square(x):
    return [{"x": E0 + x, "y": N0}, {"x": E0 + x + 10, "y": N0}, {"x": E0 + x + 10, "y": N0 + 10}, {"x": E0 + x, "y": N0 + 10}]


def entities(records):
    return [{"kind": "polygon", "id": i, "layerId": "parsel", "attrs": a, "pts": square(20 * i)} for i, a in records.items()]


def setup(indexes=(), records=None, forms=None, sources=None):
    settings = dict(SETTINGS)
    if forms:
        settings["forms"] = forms
    if sources:
        settings["sources"] = sources
    return {"format": "kentos.document", "version": 1, "name": "Dizinler", "settings": settings,
            "origin": {"x": E0, "y": N0}, "layers": layers(indexes), "activeLayer": "parsel",
            "entities": entities(RECORDS if records is None else records), "styles": {"items": [], "categories": []}}


class Refused(Exception):
    def __init__(self, code, message, path, uids=None, index=None):
        super().__init__(message)
        self.error = {"code": code, "message": message, "path": path}
        if uids:
            self.error["uids"] = uids
        self.index = index


# ---- The indexes (§1, §2) ----

def field_of(k):
    return next((f for f in FIELDS if key(f) == k), None)


def problem(indexes):
    """The contract's `layer_indexes_problem` over FIELDS, with its words."""
    if len(indexes) > 64:
        return "", f"Bir katmanın en çok 64 dizini olur; {len(indexes)} var."

    def bad(s):
        return not s.strip(JS_SPACE) or s.strip(JS_SPACE) != s or len(s) > 64 or any(
            ord(c) < 32 or 127 <= ord(c) < 160 for c in s)
    for i, x in enumerate(indexes):
        at = str(i)
        if bad(x["name"]):
            return f"{at}/name", f"“{x['name']}” bir dizin adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan."
        if any(fold(o["name"]) == fold(x["name"]) for o in indexes[:i]):
            return f"{at}/name", f"“{x['name']}” dizini iki kez var."
        fields = x.get("fields", [])
        if not fields or len(fields) > 32:
            return f"{at}/fields", f"“{x['name']}” dizininin 1–32 alanı olur; {len(fields)} var."
        for j, k in enumerate(fields):
            f = field_of(k)
            if f is None:
                return f"{at}/fields/{j}", f"“{x['name']}” dizininin “{k}” alanı katmanda yok."
            if k in fields[:j]:
                return f"{at}/fields/{j}", f"“{x['name']}” dizininde “{label(f)}” alanı iki kez var."
            if f["kind"] == "binary":
                return f"{at}/fields/{j}", (f"“{label(f)}” alanı bayt tutar; dizine girmez (ArcGIS de BLOB alanına dizin "
                                            "kurmaz).")
        if x.get("nullsEqual") and not x.get("unique"):
            return f"{at}/nullsEqual", f"“{x['name']}” dizini tekil değil; boşların aynı sayılması yalnız tekil dizinin olur."
        o = next((o for o in indexes[:i] if o["fields"] == fields), None)
        if o is not None:
            return f"{at}/fields", f"“{x['name']}” dizini “{o['name']}” dizininin alanlarını aynı sırayla tutar; birini bırakın."
    return None


def name_of_field(k):
    return field_of(k)["name"]


def values(x, attrs):
    """The index's values in a record: none when a value is empty and the index does not take empty ones as the same."""
    out = []
    for k in x["fields"]:
        v = attrs.get(name_of_field(k))
        out.append(v if v is not None and v.strip(JS_SPACE) else None)
    if not x.get("nullsEqual") and any(v is None for v in out):
        return None
    return tuple(out)


def said(x, vals):
    words = ["boş" if v is None else f"“{v}”" for v in vals]
    return words[0] if len(words) == 1 else f"({', '.join(words)})"


def stricter(x, before):
    return x.get("unique") and not any(
        b["fields"] == x["fields"] and b.get("unique") and (b.get("nullsEqual") or not x.get("nullsEqual")) for b in before)


def duplicates(x, records):
    """The first values two or more records hold under `x`, with their slots in the drawing's order."""
    seen, first = {}, []
    for slot, attrs in records.items():
        v = values(x, attrs)
        if v is None:
            continue
        seen.setdefault(v, []).append(slot)
        if len(seen[v]) == 2:
            first.append(v)
    if not first:
        return None
    return first[0], seen[first[0]]


def unique(indexes, writes, records):
    """§3: writes (slot or None, before, after, at) against the unique indexes; `records` every record now by slot."""
    written = {w[0] for w in writes if w[0] is not None}
    for x in indexes:
        if not x.get("unique"):
            continue
        names = [name_of_field(k) for k in x["fields"]]

        def changed(w):
            return w[1] is None or any(w[1].get(n) != w[2].get(n) for n in names)
        if not any(changed(w) and values(x, w[2]) is not None for w in writes):
            continue
        counts = {}
        for slot, attrs in records.items():
            if slot not in written:
                v = values(x, attrs)
                if v is not None:
                    counts[v] = counts.get(v, 0) + 1
        for w in writes:
            v = values(x, w[2])
            if v is not None:
                counts[v] = counts.get(v, 0) + 1
        for w in writes:
            v = values(x, w[2])
            if changed(w) and v is not None and counts[v] > 1:
                fields = [field_of(k) for k in x["fields"]]
                vals = [w[2].get(n) if w[2].get(n) is not None and w[2].get(n).strip(JS_SPACE) else None for n in names]
                if len(fields) == 1:
                    message = (f"“{label(fields[0])}” alanı katmanda tekil (“{x['name']}” dizini); {said(x, vals)} başka bir "
                               "kayıtta var. Başka bir değer verin.")
                else:
                    message = (f"{', '.join(f'“{label(f)}”' for f in fields)} alanları katmanda birlikte tekil "
                               f"(“{x['name']}” dizini); {said(x, vals)} başka bir kayıtta var. Başka değerler verin.")
                at = next((n for n in names if w[1] is None or w[1].get(n) != w[2].get(n)), names[0])
                raise Refused("not_unique", message, f"{w[3]}.{at}", index=x)


def values_of(given, at):
    """The values given to the layer's fields in their canonical text (empty: none), refused as the field refuses."""
    out = {}
    for k, v in given.items():
        f = next(x for x in FIELDS if x["name"] == k)
        if v is None:
            out[k] = None
            continue
        r = lf.check(f, v)
        if "error" in r:
            raise Refused("invalid_attribute", r["message"], f"{at}.{k}")
        out[k] = r["value"] or None
    return out


def node(indexes):
    return layers(indexes)[0]


def indexes_file():
    cases = []
    base = setup()

    def ok(inp, now=(), warnings=()):
        given = inp.get("indexes", [])
        assert problem(given) is None
        assert all(not stricter(x, list(now)) or duplicates(x, RECORDS) is None for x in given)
        changed = list(now) != given
        expect = {"canUndo": changed, "canRedo": False, "revision": "changed" if changed else "same",
                  "layers": layers(given)}
        if changed:
            expect["dirty"] = True
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "layer": inp["layer"], "changed": changed, "revision": "$current"}, "warnings": list(warnings)}, "expect": expect}

    def refused(name, inp, c, message, path, s=None, now=(), uids=None):
        err = {"code": c, "message": message, "path": path}
        if uids:
            err["uids"] = uids
        cases.append({"name": name, "setup": s or base, "steps": [
            {"op": "execute", "input": inp, "result": {"status": "failed", "error": err},
             "expect": {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "layers": layers(now)}}]})

    def bad(name, indexes, path, message):
        assert problem(indexes) == (path, message), (problem(indexes), path, message)
        refused(name, {"layer": "parsel", "indexes": indexes}, "invalid_indexes", message,
                "indexes" if not path else f"indexes/{path}")

    def broken(name, indexes, s=None, now=()):
        records = RECORDS
        i, x = next((i, x) for i, x in enumerate(indexes) if stricter(x, list(now)) and duplicates(x, records))
        vals, slots = duplicates(x, records)
        message = (f"“{x['name']}” dizini tekil olamaz: {said(x, vals)} {len(slots)} kayıtta var. Önce kayıtları düzeltin ya "
                   "da dizini tekil yapmayın.")
        refused(name, {"layer": "parsel", "indexes": indexes}, "index_not_unique", message, f"indexes/{i}/unique", s, now,
                [f"$uidOf:{s}" for s in slots])

    two = ok({"layer": "parsel", "indexes": [KEY, MAH]})
    cases.append({"name": "anahtar ve dizin yazılır; adımı “Dizinler”, geri alınır ve yinelenir", "setup": base, "steps": [
        two, {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": layers()}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": layers([KEY, MAH])}}]})
    again = ok({"layer": "parsel", "indexes": [KEY, MAH]}, now=[KEY, MAH])
    again["expect"]["canUndo"] = True
    cases.append({"name": "aynı dizinler yeniden: değişmez", "setup": base, "steps": [two, again]})
    gone = ok({"layer": "parsel"}, now=[KEY, MAH])
    cases.append({"name": "boş liste dizinleri kaldırır", "setup": base, "steps": [two, gone]})
    cases.append({"name": "dizinsiz katmana boş liste: değişmez", "setup": base, "steps": [ok({"layer": "parsel"})]})
    cases.append({"name": "aranan dizin yinelenen değerlerde de kurulur", "setup": base,
                  "steps": [ok({"layer": "parsel", "indexes": [MAH]})]})
    cases.append({"name": "boşları aynı sayan tekil dizin, kimliğiyle anılan alanda", "setup": base,
                  "steps": [ok({"layer": "parsel", "indexes": [BINA]})]})
    rev = {"name": "Parsel ve ada", "fields": ["Parsel", "Ada"], "unique": True}
    cases.append({"name": "aynı alanlar başka sırayla başka bir dizindir", "setup": base,
                  "steps": [ok({"layer": "parsel", "indexes": [KEY, rev]})]})
    planned = node([KEY])
    cases.append({"name": "plan katmanı yazılacak hâliyle gösterir; doğrulama", "setup": base, "steps": [
        {"op": "plan", "input": {"layer": "parsel", "indexes": [KEY]}, "result": {"status": "completed", "output": {
            "node": planned, "changed": True, "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "dirty": False, "revision": "same", "layers": layers()}},
        {"op": "validate", "input": {"layer": "parsel", "indexes": [KEY]},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same"}}]})
    first = ok({"layer": "parsel", "indexes": [MAH]})
    cases.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", "setup": base,
                  "steps": [{"op": "captureRevision", "as": "once"}, first, {
                      "op": "execute", "input": {"layer": "parsel", "indexes": [KEY], "expectedRevision": "$once"},
                      "result": {"status": "conflict", "error": {
                          "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                          "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                          "revision": "$current"}},
                      "expect": {"revision": "same", "layers": layers([MAH])}}]})
    refused("geçersiz beklenen sürüm", {"layer": "parsel", "indexes": [KEY], "expectedRevision": "x"}, "invalid_revision",
            "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.", "expectedRevision")
    refused("çizimde olmayan katman", {"layer": "yok", "indexes": [KEY]}, "layer_not_found",
            "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")
    refused("grup", {"layer": "grup", "indexes": [KEY]}, "not_a_layer",
            "“Mahalleler” bir katman grubu; dizinler yalnız katmanın olur. Grubun bir katmanını verin.", "layer")
    refused("servis katmanı", {"layer": "altlik", "indexes": [KEY]}, "service_layer",
            "“OpenStreetMap” servisten çizilir ve nesne tutmaz; dizini yoktur.", "layer")
    x = lambda **k: {**copy.deepcopy(KEY), **k}  # noqa: E731
    bad("boşluklu ad", [x(name=" Ada ve parsel")], "0/name",
        "“ Ada ve parsel” bir dizin adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.")
    bad("Türkçe katlanınca aynı ad", [x(name="İl dizini"), {"name": "il dizini", "fields": ["Mahalle"]}], "1/name",
        "“il dizini” dizini iki kez var.")
    bad("alansız dizin", [x(fields=[])], "0/fields", "“Ada ve parsel” dizininin 1–32 alanı olur; 0 var.")
    bad("33 alan", [x(fields=["Ada"] * 33)], "0/fields", "“Ada ve parsel” dizininin 1–32 alanı olur; 33 var.")
    bad("katmanda olmayan alan", [x(fields=["Ada", "Renk"])], "0/fields/1", "“Ada ve parsel” dizininin “Renk” alanı katmanda yok.")
    bad("kimliği olan alan adıyla anılmaz", [{"name": "Bina", "fields": ["Bina no"]}], "0/fields/0",
        "“Bina” dizininin “Bina no” alanı katmanda yok.")
    bad("aynı alan iki kez", [x(fields=["Ada", "Ada"])], "0/fields/1", "“Ada ve parsel” dizininde “Ada” alanı iki kez var.")
    bad("bayt alanı", [{"name": "Belge", "fields": ["Belge"]}], "0/fields/0",
        "“Belge” alanı bayt tutar; dizine girmez (ArcGIS de BLOB alanına dizin kurmaz).")
    bad("tekil olmayan dizinde boşlar aynı", [{"name": "Mahalle", "fields": ["Mahalle"], "nullsEqual": True}], "0/nullsEqual",
        "“Mahalle” dizini tekil değil; boşların aynı sayılması yalnız tekil dizinin olur.")
    bad("aynı alanlar aynı sırayla iki dizinde", [KEY, {"name": "Yine ada", "fields": ["Ada", "Parsel"]}], "1/fields",
        "“Yine ada” dizini “Ada ve parsel” dizininin alanlarını aynı sırayla tutar; birini bırakın.")
    many = [{"name": f"Dizin {i}", "fields": ["Ada"] if i == 0 else ["Mahalle", "Ada"] if i == 1 else
             [["Ada", "Parsel", "Mahalle", "bina-no", "Alan"][j] for j in range(5) if (i >> j) & 1]} for i in range(65)]
    bad("65 dizin", many, "", "Bir katmanın en çok 64 dizini olur; 65 var.")
    broken("kayıtların yinelediği değerde tekil dizin kurulmaz", [MAH, {"name": "Mahalle tekil", "fields": ["Mahalle", "Alan"],
                                                                         "unique": True, "nullsEqual": True}])
    broken("tek alanlı anahtar: iki kayıtta aynı ada", [{"name": "Ada", "fields": ["Ada"], "unique": True}])
    broken("boşlar aynı sayılınca parselsiz iki kayıt bozar", [x(nullsEqual=True)])
    kept_bad = {"name": "Ada", "fields": ["Ada"], "unique": True}
    now = [kept_bad]
    keep = ok({"layer": "parsel", "indexes": [kept_bad, MAH]}, now=now)
    keep["expect"]["canUndo"] = True
    cases.append({"name": "kayıtların zaten bozduğu tekil dizin olduğu gibi kalır; yanına dizin eklenir",
                  "setup": setup(now), "steps": [keep]})
    broken("var olan tekil dizine boşları aynı saymak daha sıkıdır", [x(nullsEqual=True)], setup([KEY]), [KEY])
    warn = [{"code": "index_not_in_source", "message": "“Parsel” bir tabloya bağlı: dizinleri KentOS denetler, tabloda "
             "kurulmaz. Tablonun kendi tekil kısıtları yine sorulur.", "path": "indexes"}]
    bound = setup(sources=[{"id": "kaynak-1", "kind": "geopackage", "path": "/yok/tapu.gpkg", "table": "parsel",
                            "layer": "parsel", "revision": "0" * 64, "digest": "f" * 64, "srid": 5256, "writable": True}])
    told = ok({"layer": "parsel", "indexes": [KEY]}, warnings=warn)
    cases.append({"name": "bağlı katmanda dizin: KentOS denetler, tabloda kurulmaz", "setup": bound, "steps": [told]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.indexes", "commandVersion": 1,
            "title": "Dizinler",
            "note": "ADR 0758 §1, §2: katmanın dizinleri ve anahtarları. Written by scripts/fixtures/layer_indexes_cases.py "
                    "without KentOS code; do not edit by hand.", "setup": base, "cases": cases}


# ---- cad.entities.set ----

def set_file():
    cases = []
    INDEXES = [KEY, MAH, BINA]

    def step(uids, attrs, now=None, undo=False):
        now = copy.deepcopy(now or RECORDS)
        after_all = copy.deepcopy(now)
        inp = {"uids": [f"$uidOf:{u}" for u in uids], "attrs": attrs, "operation": "attributes"}
        try:
            given = values_of(attrs, "attrs")
            writes = []
            for u in uids:
                after = dict(now[u])
                for k, v in given.items():
                    if v is None:
                        after.pop(k, None)
                    else:
                        after[k] = v
                if after == now[u]:
                    continue
                writes.append((u, now[u], after, "attrs"))
                after_all[u] = after
            unique(INDEXES, writes, now)
        except Refused as r:
            expect = {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in now.items()},
                      "canUndo": undo, "canRedo": False, "revision": "same"}
            if not undo:
                expect["dirty"] = False
            return {"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error}, "expect": expect}, now
        changed = [u for u in uids if after_all[u] != now[u]]
        expect = {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in after_all.items()},
                  "canUndo": bool(changed) or undo, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "changed": [f"$uidOf:{u}" for u in changed], "ids": changed, "revision": "$current"}, "warnings": []},
            "expect": expect}, after_all

    def case(name, uids, attrs):
        s, _ = step(uids, attrs)
        cases.append({"name": name, "steps": [s]})

    case("ada değişince (ada, parsel) başka kayıtta var: yeri Ada", [3], {"Ada": "101"})
    case("parsel değişince (ada, parsel) başka kayıtta var: yeri Parsel", [1], {"Parsel": "6"})
    case("ikisi birden değişir, başka kaydın değerleri: yeri ilk değişen alan Ada", [3], {"Ada": "101", "Parsel": "6"})
    case("yeni (ada, parsel): yazılır", [3], {"Parsel": "7"})
    case("parseli boş kayda parsel verilir", [4], {"Parsel": "5"})
    case("iki kayda birden aynı (ada, parsel): ikisi de yazılmaz", [4, 5], {"Parsel": "7"})
    case("parsel boşalınca anahtar sorulmaz (boş değer değildir)", [1], {"Parsel": None})
    case("parseli boş iki kaydın adası aynı kalır: boşlar farklı", [4, 5], {"Mahalle": "Yalı"})
    case("anahtarın alanlarına dokunmayan yazma sorulmaz", [1, 2], {"Mahalle": "Çarşı"})
    case("kendi değerlerini yeniden yazmak serbest", [1], {"Ada": "101", "Parsel": "5"})
    case("aranan dizin sınırlamaz: aynı mahalle", [3], {"Mahalle": "Merkez"})
    case("boşları aynı sayan dizinde boşalan bina no, boş kayıt varken: reddedilir", [1], {"Bina no": None})
    case("boşları aynı sayan dizinde başka kaydın bina nosu", [2], {"Bina no": "B1"})
    case("boşları aynı sayan dizinde yeni bina no", [6], {"Bina no": "B6"})
    case("boş kayda başka kaydın (ada, parsel)'i", [6], {"Ada": "102", "Parsel": "5"})
    case("boş kayda yeni (ada, parsel)", [6], {"Ada": "104", "Parsel": "1"})
    case("ada sayının yazımıyla: 0101 101'dir", [3], {"Ada": "0101", "Parsel": "6"})
    case("iki kaydın adası birden 101: (101, 5) ve (101, 6) zaten var mı? 3 ve 5 için", [3, 5], {"Ada": "101"})
    one, after = step([3], {"Parsel": "9"})
    two, _ = step([1], {"Ada": "102", "Parsel": "9"}, now=after, undo=True)
    cases.append({"name": "önce yeni değer, sonra başka kayıt onu alamaz", "steps": [one, two]})
    one, after = step([1], {"Parsel": "8"})
    two, _ = step([2], {"Parsel": "5"}, now=after, undo=True)
    cases.append({"name": "değeri bırakılan (ada, parsel) başka kayda geçer", "steps": [one, two]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.set", "commandVersion": 1,
            "title": "Öznitelikler ve katmanın anahtarı",
            "note": "ADR 0758 §3: tekil dizinin alanlarını değiştiren yazma, alanlarında başka bir kaydın değerlerini verirse "
                    "reddedilir; boş değer değer değildir, boşları aynı sayan dizin onları da karşılaştırır. Written by "
                    "scripts/fixtures/layer_indexes_cases.py without KentOS code.",
            "setup": setup(INDEXES), "cases": cases}


# ---- cad.polygon.create ----

def create_file():
    cases = []
    INDEXES = [KEY, BINA]
    existing = {1: RECORDS[1], 6: RECORDS[6]}

    def made(name, attrs=None):
        inp = {"layerId": "parsel", "pts": square(200)}
        try:
            out = {}
            if attrs:
                inp["attrs"] = attrs
                out = {k: v for k, v in values_of(attrs, "attrs").items() if v}
            unique(INDEXES, [(None, None, out, "attrs")], existing)
        except Refused as r:
            cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error},
                                                   "expect": {"ids": [1, 6], "canUndo": False, "dirty": False,
                                                              "revision": "same"}}]})
            return
        expect = {"ids": [1, 6, 7], "canUndo": True, "dirty": True, "revision": "changed"}
        if out:
            expect["fields"] = {"7": {"attrs": out}}
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": {"uid": "$uid", "id": 7, "revision": "$current"}, "warnings": []},
            "expect": expect}]})

    made("başka kaydın (ada, parsel)'i: yeri Ada", {"Ada": "101", "Parsel": "5", "Bina no": "B9"})
    made("yeni (ada, parsel)", {"Ada": "101", "Parsel": "7", "Bina no": "B9"})
    made("aynı adada başka parsel", {"Ada": "101", "Parsel": "6", "Bina no": "B9"})
    made("aynı parsel başka adada", {"Ada": "102", "Parsel": "5", "Bina no": "B9"})
    made("parselsiz yeni kayıt (boş değer değildir)", {"Ada": "101", "Bina no": "B9"})
    made("adasız yeni kayıt", {"Parsel": "5", "Bina no": "B9"})
    made("değersiz kayıt: boş kayıt varken bina no boşları aynı", None)
    made("bina nosuz yeni kayıt: boş kayıt varken boşlar aynı", {"Ada": "105", "Parsel": "1"})
    made("boşluklu bina no boş sayılır", {"Ada": "105", "Parsel": "1", "Bina no": "   "})
    made("başka kaydın bina nosu", {"Ada": "105", "Parsel": "1", "Bina no": "B1"})
    made("bina nosu büyük küçük harf ayrı", {"Ada": "105", "Parsel": "1", "Bina no": "b1"})
    made("bina nosu sonunda boşlukla başka değer", {"Ada": "105", "Parsel": "1", "Bina no": "B1 "})
    made("sayının yazımıyla aynı (ada, parsel)", {"Ada": "+101", "Parsel": "05", "Bina no": "B9"})
    made("sayının yazımıyla yeni (ada, parsel)", {"Ada": "0101", "Parsel": "+7", "Bina no": "B9"})
    made("mahalle ve alan anahtarda değil", {"Ada": "106", "Parsel": "1", "Mahalle": "Merkez", "Alan": "250", "Bina no": "B7"})
    made("bina no boş kayıttaki gibi başka alanlarla", {"Alan": "250"})
    made("alanı aynı, anahtarı yeni", {"Ada": "107", "Parsel": "2", "Alan": "250", "Bina no": "B8"})
    made("büyük ada numarası", {"Ada": "9007199254740993", "Parsel": "1", "Bina no": "B10"})
    made("eksi ada numarası", {"Ada": "-101", "Parsel": "5", "Bina no": "B11"})
    made("başka kaydın (ada, parsel)'i bina no yeniyken de", {"Ada": "101", "Parsel": "5", "Bina no": "B12"})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.polygon.create", "commandVersion": 1,
            "title": "Alan çiz ve katmanın anahtarı",
            "note": "ADR 0758 §3: yeni kayıt anahtarın değerlerini yineleyemez. Written by scripts/fixtures/layer_indexes_cases.py "
                    "without KentOS code.", "setup": setup(INDEXES, existing), "cases": cases}


# ---- cad.fields.write ----

def write_file():
    """A batch of values (Yapıştır, docs/adr/0507) a key refuses some of: the writes holding the values that clash are
    taken out, each told, until the rest fit; policy `valid` writes the rest."""
    cases = []
    INDEXES = [KEY, BINA]

    def batch(name, rows, policy=None):
        now = copy.deepcopy(RECORDS)
        wants = []
        for slot, given in rows:
            after = dict(now[slot])
            for k, v in values_of(given, "values").items():
                if v is None:
                    after.pop(k, None)
                else:
                    after[k] = v
            wants.append((slot, now[slot], after))
        changed = [(s, b, a) for s, b, a in wants if a != b]
        problems = []
        while True:
            writes = [(s, b, a, "attrs") for s, b, a in changed]
            try:
                unique(INDEXES, writes, now)
                break
            except Refused as r:
                field = r.error["path"].rsplit(".", 1)[-1]
                x = r.index
                counts = {}
                written = {s for s, _, _ in changed}
                for slot, attrs in now.items():
                    if slot not in written:
                        v = values(x, attrs)
                        if v is not None:
                            counts[v] = counts.get(v, 0) + 1
                for s, _, a in changed:
                    v = values(x, a)
                    if v is not None:
                        counts[v] = counts.get(v, 0) + 1
                hit = [i for i, (s, b, a) in enumerate(changed) if values(x, a) is not None and counts[values(x, a)] > 1]
                for i in reversed(hit):
                    s, b, a = changed.pop(i)
                    problem = {"uid": f"$uidOf:{s}", "field": field, "code": r.error["code"], "message": r.error["message"]}
                    if a.get(field) is not None:
                        problem["value"] = a[field]
                    problems.append((s, problem))
                if not changed:
                    break
        order = {s: i for i, (s, _) in enumerate(rows)}
        problems.sort(key=lambda p: order[p[0]])
        inp = {"layer": "parsel", "write": {"kind": "values", "rows": [
            {"uid": f"$uidOf:{s}", "values": g} for s, g in rows]}}
        if policy:
            inp["policy"] = policy
        said = f" İlki: {problems[0][1]['message']} ({problems[0][1]['field']})." if problems else ""
        if problems and policy != "valid":
            cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
                "code": "invalid_values", "message": f"{len(problems)} nesnenin değeri alanların kurallarına uymuyor; hiçbir "
                f"değer yazılmadı.{said} Sorunların hepsi plan'da; uyanları yazmak için policy valid verin.", "path": "write"}},
                "expect": {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in RECORDS.items()},
                           "canUndo": False, "dirty": False, "revision": "same"}}]})
            return
        changes = []
        for s, b, a in changed:
            for k in sorted(set(b) | set(a)):
                if b.get(k) != a.get(k):
                    change = {"uid": f"$uidOf:{s}", "field": k}
                    if b.get(k) is not None:
                        change["before"] = b[k]
                    if a.get(k) is not None:
                        change["after"] = a[k]
                    changes.append(change)
        after_all = copy.deepcopy(RECORDS)
        for s, _, a in changed:
            after_all[s] = a
        output = {"changed": len(changed), "unchanged": len([1 for s, b, a in wants if a == b]), "kept": 0,
                  "invalid": len(problems), "revision": "$current"}
        if changes:
            output["changes"] = changes
        if problems:
            output["problems"] = [p for _, p in problems]
        expect = {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in after_all.items()},
                  "canUndo": bool(changed), "revision": "changed" if changed else "same"}
        if changed:
            expect["dirty"] = True
        warnings = [{"code": "values_skipped", "message": f"{len(problems)} nesnenin değeri kurallara uymadığı için "
                     f"yazılmadı.{said}", "path": "write"}] if problems else []
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": output, "warnings": warnings}, "expect": expect}]})

    batch("iki kaydın parseli yer değiştirir: anahtar tutar", [(1, {"Parsel": "6"}), (2, {"Parsel": "5"})])
    batch("uyanları yaz: (ada, parsel)'i başka kayıtta olanlar alınmaz, öbürleri yazılır",
          [(3, {"Ada": "101"}), (4, {"Parsel": "1"}), (5, {"Parsel": "1"}), (6, {"Ada": "200", "Parsel": "1"})], "valid")
    batch("yalnız anahtarı aynı değerleri yineleyenler alınmaz; ada aynı, parseli başka olan yazılır",
          [(4, {"Parsel": "2"}), (5, {"Parsel": "3"}), (6, {"Ada": "103", "Parsel": "2"})], "valid")
    batch("hepsi ya da hiç: biri anahtarı yinelerse hiçbiri yazılmaz",
          [(3, {"Ada": "101"}), (6, {"Ada": "200", "Parsel": "1"})])
    batch("iki kaydın bina nosu yer değiştirir", [(1, {"Bina no": "B2"}), (2, {"Bina no": "B1"})])
    batch("hepsi ya da hiç: iki satır aynı (ada, parsel)'i verir", [(4, {"Parsel": "1"}), (5, {"Parsel": "1"})])
    batch("uyanları yaz: aynı (ada, parsel)'i veren iki satır alınmaz", [(4, {"Parsel": "1"}), (5, {"Parsel": "1"}),
                                                                          (6, {"Mahalle": "Yalı"})], "valid")
    batch("boşları aynı sayan bina no: iki satır birden boşalır", [(1, {"Bina no": None}), (2, {"Bina no": None})], "valid")
    batch("boşalan bina no boş kayıtla çakışır; öbür satır yazılır", [(1, {"Bina no": None}), (2, {"Mahalle": "Çarşı"})],
          "valid")
    batch("boş kayda bina no verilir, öbürü boşalır: boşlar yine tek", [(6, {"Bina no": "B6"}), (1, {"Bina no": None})])
    batch("anahtara dokunmayan satırlar sorulmaz", [(1, {"Mahalle": "Çarşı"}), (2, {"Mahalle": "Çarşı"})])
    batch("parsel boşaltılır: anahtar sorulmaz", [(1, {"Parsel": None}), (2, {"Parsel": None})])
    batch("zincir: 1 parseli 7'ye, 2 parseli 5'e", [(1, {"Parsel": "7"}), (2, {"Parsel": "5"})])
    batch("zincirin ucu başka kayıtta: 2 parseli 5'e ama 1 değişmez", [(2, {"Parsel": "5"})])
    batch("uyanları yaz: bir satırda iki anahtar birden bozulur, ilki söylenir",
          [(2, {"Parsel": "5", "Bina no": "B1"}), (6, {"Ada": "300", "Parsel": "1", "Bina no": "B6"})], "valid")
    batch("uyanları yaz: hepsi çakışırsa hiçbiri yazılmaz, değişmez",
          [(3, {"Ada": "101"}), (4, {"Ada": "101", "Parsel": "6"})], "valid")
    batch("sayının yazımıyla aynı değer: (+101, 05) (101, 5)'tir", [(3, {"Ada": "+101", "Parsel": "05"})], "valid")
    batch("yeni anahtarlar üç satırda", [(4, {"Parsel": "1"}), (5, {"Parsel": "2"}), (6, {"Ada": "104", "Parsel": "1"})])
    batch("aynı değeri yeniden yazmak değişiklik değildir", [(1, {"Ada": "101", "Parsel": "5"})])
    batch("boşluklu bina no boş sayılır ve boş kayıtla çakışır", [(3, {"Bina no": "  "})], "valid")
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.fields.write", "commandVersion": 1,
            "title": "Değerleri yaz ve katmanın anahtarı",
            "note": "ADR 0758 §3: toplu yazmada anahtarın değerlerini yineleyen kayıtlar alınmaz, yalnız onlar. Written by "
                    "scripts/fixtures/layer_indexes_cases.py without KentOS code.", "setup": setup(INDEXES), "cases": cases}


# ---- cad.forms.validate ----

BODY = {"revision": 1, "items": [
    {"type": "field", "id": "parsel", "field": "Parsel"},
    {"type": "field", "id": "ada", "field": "Ada"},
    {"type": "field", "id": "mahalle", "field": "Mahalle"},
    {"type": "field", "id": "bina", "field": "bina-no"},
]}
FORM = {"id": "parsel-formu", "name": "Parsel", "layer": "parsel", "draft": BODY,
        "releases": [{"version": 1, "digest": fc.digest(BODY), "body": BODY}]}


def index_issues(indexes, body, attrs, others):
    """The form's key issues: one per unique index another record holds the values of, on its first field the form
    shows (its first field when the form shows none)."""
    out = []
    shown = [x["field"] for x in body["items"] if x["type"] == "field"]
    for x in indexes:
        if not x.get("unique"):
            continue
        mine = values(x, attrs)
        if mine is None:
            continue
        if not any(values_or_raw(x, o) == mine for o in others):
            continue
        at = next((k for k in x["fields"] if k in shown), x["fields"][0])
        item = next((i["id"] for i in body["items"] if i.get("field") == at), None)
        out.append(fc.issue("form_unique", True, item, at, "index/unique"))
    return out


def values_or_raw(x, attrs):
    """Another record's values as the index compares them, empty ones kept (a set equal to this one's)."""
    out = []
    for k in x["fields"]:
        v = attrs.get(name_of_field(k))
        out.append(v if v is not None and v.strip(JS_SPACE) else None)
    return tuple(out)


def validate_file():
    cases = []
    INDEXES = [KEY, BINA]
    S = setup(INDEXES, forms=[FORM])

    def run(name, values=None, slot=3):
        f = fc.Form(BODY, FIELDS, ())
        obj = {"length": 40, "area": 100, "vertices": 4}
        after, found = f.assess(dict(RECORDS[slot]), fc.utf8_sorted(dict(values or {})), obj, False)
        others = [a for s, a in RECORDS.items() if s != slot]
        issues = fc.preflight(BODY, FIELDS, ()) + found + f.unique_issues(after, obj, others) + index_issues(
            INDEXES, BODY, after, others)
        out = {"issues": fc.shown_issues(issues), "valid": not fc.errors(issues), "items": f.described(after, obj),
               "version": 1, "attrs": after}
        fc.with_requires(out, BODY)
        inp = {"form": "parsel-formu", "uid": f"$uidOf:{slot}"}
        if values:
            inp["values"] = values
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp,
                                               "result": {"status": "completed", "output": out, "warnings": []},
                                               "expect": {"ids": list(RECORDS), "canUndo": False, "canRedo": False,
                                                          "dirty": False, "revision": "same"}}]})

    run("geçerli kayıt: sorun yok", slot=1)
    run("başka kaydın (ada, parsel)'i: anahtarın formda gösterilen ilk alanı Ada'da", {"Ada": "101"})
    run("yeni (ada, parsel)", {"Ada": "101", "Parsel": "9"})
    run("parsel boşalınca anahtar sorulmaz", {"Ada": "101", "Parsel": ""})
    run("boşları aynı sayan bina no: boş kayıt varken boşaltılır", {"bina-no": ""}, slot=2)
    run("başka kaydın bina nosu", {"bina-no": "B1"}, slot=2)
    run("aranan dizinin alanı sınırlamaz", {"Mahalle": "Merkez"})
    run("iki anahtar birden bozulur: ikisi de söylenir", {"Ada": "101", "Parsel": "6", "bina-no": "B1"})
    run("parsel değişir, adası aynı başka kayıt", {"Parsel": "6"}, slot=1)
    run("kendi değerleri: sorun yok", {"Ada": "101", "Parsel": "5"}, slot=1)
    run("boş kayıt: bina nosu boş, başka boş yok", slot=6)
    run("boş kayda başka kaydın (ada, parsel)'i", {"Ada": "102", "Parsel": "5"}, slot=6)
    run("boş kayda yeni bina no", {"bina-no": "B6"}, slot=6)
    run("parselsiz iki kayıt: boşlar farklı", slot=4)
    run("parselsiz kayda başka kaydın parseli", {"Parsel": "5"}, slot=4)
    run("parselsiz kayda yeni parsel", {"Parsel": "1"}, slot=4)
    run("sayının yazımıyla aynı (ada, parsel)", {"Ada": "+101", "Parsel": "06"}, slot=1)
    run("bina no büyük küçük harf ayrı", {"bina-no": "b1"}, slot=2)
    run("bina no boşlukla boş sayılır", {"bina-no": "   "}, slot=2)
    run("mahalle boşaltılır", {"Mahalle": ""}, slot=1)
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.validate", "commandVersion": 1,
            "title": "Formu denetle ve katmanın anahtarı",
            "note": "ADR 0758 §3: form katmanın tekil dizinlerini dizinin formda gösterilen ilk alanında söyler. Written by "
                    "scripts/fixtures/layer_indexes_cases.py with the form twin of forms_cases.py, without KentOS code; the "
                    "issues' sentences are the core's ($any).", "setup": S, "cases": cases}


def outputs():
    return {OUT / "cad.layers.indexes.json": indexes_file(), OUT / "cad.entities.set.indexes.json": set_file(),
            OUT / "cad.polygon.create.indexes.json": create_file(), OUT / "cad.fields.write.indexes.json": write_file(),
            OUT / "cad.forms.validate.indexes.json": validate_file(),
            OUT / "cad.records.duplicate.indexes.json": duplicate_file()}


# ---- cad.records.duplicate ----

DUP_RECORDS = {**copy.deepcopy(RECORDS), 6: {"Alan": "250.00", "Bina no": "B6"}}
YAPI = [{"name": "Kod", "kind": "text", "unique": True}, {"name": "Kat", "kind": "integer"}]
DUZ = [{"name": "Ada", "kind": "integer"}, {"name": "Parsel", "kind": "integer"}]


def dup_setup():
    s = setup([KEY, MAH, BINA], DUP_RECORDS)
    s["layers"].insert(1, layer("yapi", "Yapı", YAPI))
    s["layers"].insert(2, layer("duz", "Düz", DUZ, indexes=[{"name": "Ada", "fields": ["Ada"]}]))
    s["entities"] += [
        {"kind": "polygon", "id": 7, "layerId": "yapi", "attrs": {"Kod": "Y-1", "Kat": "2"}, "pts": square(140)},
        {"kind": "polygon", "id": 8, "layerId": "yapi", "attrs": {"Kat": "1"}, "pts": square(160)},
        {"kind": "polygon", "id": 9, "layerId": "duz", "attrs": {"Ada": "101", "Parsel": "5"}, "pts": square(180)},
        {"kind": "polygon", "id": 10, "layerId": "duz", "attrs": {"Ada": "102"}, "pts": square(200)},
    ]
    return s


def duplicate_file():
    """Kaydı çoğalt (docs/adr/0415) over the layers' keys: a copy leaves empty what the layer keeps unique (a field,
    docs/adr/0438 §3) and what its unique indexes hold (docs/adr/0758 §3), said once each; copies whose emptied key
    another record holds (an index taking empty values as the same) are refused. No relation here: a copy is its object."""
    cases = []
    base = dup_setup()
    es0 = {e["id"]: e for e in base["entities"]}
    layers_of = {n["id"]: n for n in base["layers"]}
    locked_name = {"duz": "Düz"}

    def key_fields(layer_id):
        n = layers_of[layer_id]
        fields = n.get("fields", [])
        out = [(f"Katmanda tekil olan “{f['name']}” alanı", f["name"]) for f in fields if f.get("unique")]
        for x in n.get("indexes", []):
            if not x.get("unique"):
                continue
            for k in x["fields"]:
                f = next(f for f in fields if key(f) == k)
                if not any(name == f["name"] for _, name in out):
                    out.append((f"Katmanın “{x['name']}” anahtarındaki “{label(f)}” alanı", f["name"]))
        return out

    def lock_steps(locked):
        return [{"op": "execute", "command": "cad.layers.set", "input": {"operation": "locked", "layer": l, "value": True},
                 "result": {"status": "completed", "output": {"changed": True, "active": "parsel", "revision": "$current"},
                            "warnings": []}} for l in locked]

    def dup(name, slots, locked=(), redo=False):
        firsts = list(dict.fromkeys(slots))
        open_ = [s for s in firsts if es0[s]["layerId"] not in locked]
        skipped = len(firsts) - len(open_)
        nxt = max(es0) + 1
        copies, made, said = [], {}, []
        for s in open_:
            e = copy.deepcopy(es0[s])
            for why, field in key_fields(e["layerId"]):
                if field in e["attrs"]:
                    del e["attrs"][field]
                    if why not in said:
                        said.append(why)
            e["id"] = nxt
            made[nxt] = e
            copies.append({"from": f"$uidOf:{s}", "uid": f"$uidOf:{nxt}", "id": nxt})
            nxt += 1
        warnings = []
        if skipped:
            warnings.append({"code": "layer_locked", "message": f"{skipped} nesne kilitli katmanda olduğu için atlandı. "
                             "Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"})
        warnings += [{"code": "unique_cleared", "message": f"{why}; kopyalarda boş bırakıldı.", "path": "uids"} for why in said]
        input_ = {"uids": [f"$uidOf:{s}" for s in slots]}
        before = sorted(es0)
        # The copies against the keys, as new records.
        records = {s: e["attrs"] for s, e in es0.items()}
        try:
            for layer_id in dict.fromkeys(e["layerId"] for e in made.values()):
                indexes = layers_of[layer_id].get("indexes", [])
                if layer_id == "parsel":
                    unique(indexes, [(None, None, e["attrs"], "uids") for e in made.values() if e["layerId"] == layer_id],
                           {s: a for s, a in records.items() if es0[s]["layerId"] == layer_id})
        except Refused as r:
            cases.append({"name": name, "steps": lock_steps(locked) + [
                {"op": "execute", "input": input_, "result": {"status": "failed", "error": r.error},
                 "expect": {"ids": before}}]})
            return
        after = sorted(list(es0) + list(made))
        expect = {"ids": after, "entities": {str(k): v for k, v in made.items()}, "canUndo": True, "revision": "changed"}
        steps = lock_steps(locked) + [
            {"op": "plan", "input": input_, "result": {"status": "completed", "output": {
                "count": len(copies), "children": 0, "revision": "$current"}, "warnings": warnings}},
            {"op": "execute", "input": input_, "result": {"status": "completed", "output": {
                "copies": copies, "revision": "$current"}, "warnings": warnings}, "expect": expect},
            {"op": "undo", "returns": "Kaydı çoğalt", "expect": {"ids": before}}]
        if redo:
            steps.append({"op": "redo", "returns": "Kaydı çoğalt", "expect": {"ids": after, "entities": expect["entities"]}})
        cases.append({"name": name, "steps": steps})

    def refused(name, input_, code, message, path, locked=()):
        cases.append({"name": name, "steps": lock_steps(locked) + [
            {"op": "execute", "input": input_, "result": {"status": "failed", "error": {
                "code": code, "message": message, "path": path}}, "expect": {"ids": sorted(es0)}}]})

    dup("parselin kopyası: anahtarların alanları boş, aranan mahalle kalır; geri alınır ve yinelenir", [1], redo=True)
    dup("parselsiz kayıt: yalnız tuttuğu anahtar alanları söylenir", [4])
    dup("yalnız bina nosu olan kayıt", [6])
    dup("iki parsel birden: boşları aynı sayan bina anahtarı ikinci boşu reddeder", [1, 2])
    dup("parselsiz iki kayıt birden: yine bina anahtarı reddeder", [4, 5])
    dup("katmanda tekil kod boş bırakılır", [7])
    dup("kodu olmayan yapı: söylenecek bir şey yok", [8])
    dup("iki yapı: tekil kod bir kez söylenir", [7, 8])
    dup("yalnız aranan dizini olan katman: hiçbir alan boşaltılmaz", [9])
    dup("aranan dizinin iki kaydı", [9, 10])
    dup("yapı ve düz katman birlikte", [7, 9])
    dup("parsel ve yapı birlikte: önce parselin anahtarları, sonra yapının kodu", [1, 7])
    dup("yapı ve parsel ters sırayla", [7, 1])
    dup("aynı kimlik iki kez: bir kopya", [7, 7])
    dup("kilitli katmandaki kayıt atlanır, öbürü çoğaltılır", [9, 7], locked=("duz",))
    refused("hepsi kilitli katmanda", {"uids": ["$uidOf:9"]}, "layer_locked",
            "“Düz” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]", locked=("duz",))
    refused("boş liste", {"uids": []}, "no_entities", "Çoğaltılacak nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids")
    refused("1001 nesne", {"uids": ["$uidOf:1"] * 1001}, "invalid_count", "En çok 1000 nesne birlikte çoğaltılır; 1001 verildi.",
            "uids")
    refused("geçersiz beklenen sürüm", {"uids": ["$uidOf:1"], "expectedRevision": "on iki"}, "invalid_revision",
            "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.", "expectedRevision")
    cases.append({"name": "doğrulama: bina anahtarının reddi yazmadan söylenir", "steps": [
        {"op": "validate", "input": {"uids": ["$uidOf:1", "$uidOf:2"]}, "result": {"status": "failed", "error": {
            "code": "not_unique", "message": "“Bina no” alanı katmanda tekil (“Bina” dizini); boş başka bir kayıtta var. Başka "
            "bir değer verin.", "path": "uids.Bina no"}}, "expect": {"ids": sorted(es0)}}]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.records.duplicate", "commandVersion": 1,
            "title": "Kaydı çoğalt ve katmanın anahtarları",
            "note": "ADR 0758 §3, ADR 0438 §3: kopya katmanın tekil alanlarını ve anahtarlarının alanlarını boş bırakır; boşları "
                    "aynı sayan anahtar ikinci boşu reddeder. Written by scripts/fixtures/layer_indexes_cases.py without KentOS "
                    "code.", "setup": base, "cases": cases}


FILES = ROOT / "fixtures/layer-indexes/v1"


def files():
    """`dizinli.gpkg` written by GDAL (OGR) and its indexes by SQLite, without KentOS: a parcel table with a unique index
    over two columns, a plain one, a partial one, one over an expression, one over a BLOB, one over the fid and one whose
    name does not start with the table's; an owners' table whose column OGR made UNIQUE."""
    import sqlite3
    from osgeo import gdal, ogr, osr
    gdal.UseExceptions()
    FILES.mkdir(parents=True, exist_ok=True)
    path = FILES / "dizinli.gpkg"
    if path.exists():
        path.unlink()
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    parsel = ds.CreateLayer("parsel", srs, ogr.wkbPolygon)
    for name, kind in (("ada", ogr.OFTInteger), ("parsel", ogr.OFTInteger), ("mahalle", ogr.OFTString),
                       ("belge", ogr.OFTBinary)):
        parsel.CreateField(ogr.FieldDefn(name, kind))
    for i, (a, n, m) in enumerate(((101, 5, "Merkez"), (101, 6, "Merkez"), (102, 5, None))):
        f = ogr.Feature(parsel.GetLayerDefn())
        f["ada"], f["parsel"] = a, n
        if m:
            f["mahalle"] = m
        x = E0 + 20 * i
        f.SetGeometry(ogr.CreateGeometryFromWkt(
            f"POLYGON (({x} {N0},{x + 10} {N0},{x + 10} {N0 + 10},{x} {N0 + 10},{x} {N0}))"))
        parsel.CreateFeature(f)
    malik = ds.CreateLayer("malik", None, ogr.wkbNone)
    tc = ogr.FieldDefn("tc", ogr.OFTString)
    tc.SetUnique(True)
    malik.CreateField(tc)
    malik.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for t, a in (("10000000146", "Ayşe"), ("10000000078", "Mehmet")):
        f = ogr.Feature(malik.GetLayerDefn())
        f["tc"], f["ad"] = t, a
        malik.CreateFeature(f)
    ds = None
    con = sqlite3.connect(path)
    con.executescript("""
        CREATE UNIQUE INDEX "parsel_Ada ve parsel" ON parsel (ada, parsel);
        CREATE INDEX "parsel_Mahalle" ON parsel (mahalle);
        CREATE INDEX "parsel_kismi" ON parsel (ada) WHERE ada > 100;
        CREATE INDEX "parsel_ifade" ON parsel (lower(mahalle));
        CREATE INDEX "parsel_belge" ON parsel (belge);
        CREATE INDEX "parsel_fid_ada" ON parsel (fid, ada);
        CREATE INDEX "baska_ad" ON parsel (parsel);
    """)
    con.commit()
    con.close()
    print(f"{path.relative_to(ROOT)} yazıldı")


def written() -> int:
    """KentOS's `kentos-dizinler.gpkg` (KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all
    layer_indexes) read with SQLite alone: the layer's three indexes over the columns its fields went to, the key unique
    (a duplicate row refused), the plain one plain, the one taking empty values as the same written unique (SQLite takes
    every NULL as different)."""
    import sqlite3
    path = FILES / "out/kentos-dizinler.gpkg"
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    table = con.execute("SELECT table_name FROM gpkg_contents WHERE identifier = 'Parsel' OR table_name = 'Parsel'").fetchone()
    assert table, "Parsel tablosu yok"
    table = table[0]
    listed = {name: (unique, origin, partial) for _, name, unique, origin, partial in
              con.execute(f'PRAGMA index_list("{table}")')}
    want = {f"{table}_Ada ve parsel": (1, ["Ada", "Parsel"]), f"{table}_Mahalle": (0, ["Mahalle"]),
            f"{table}_Bina": (1, ["Bina no"])}
    problems = []
    for name, (unique, cols) in want.items():
        if name not in listed:
            problems.append(f"{name} yok: {sorted(listed)}")
            continue
        if listed[name] != (unique, "c", 0):
            problems.append(f"{name}: {listed[name]}, beklenen ({unique}, 'c', 0)")
        got = [row[2] for row in con.execute(f'PRAGMA index_info("{name}")')]
        if got != cols:
            problems.append(f"{name}: sütunlar {got}, beklenen {cols}")
    con.close()
    # The key refuses a second (101, 5), in a copy.
    import shutil
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        copy_path = Path(tmp) / "kopya.gpkg"
        shutil.copy(path, copy_path)
        con = sqlite3.connect(copy_path)
        # The table's spatial index triggers name GeoPackage's functions; a row without a geometry calls none of them.
        for name in ("ST_IsEmpty", "ST_MinX", "ST_MaxX", "ST_MinY", "ST_MaxY"):
            con.create_function(name, 1, lambda g: None)
        try:
            con.execute(f'INSERT INTO "{table}" ("Ada", "Parsel") VALUES (101, 5)')
            problems.append("(101, 5) ikinci kez yazılabildi: dizin tekil değil")
        except sqlite3.IntegrityError:
            pass
        con.close()
    for p in problems:
        print(p, file=sys.stderr)
    if not problems:
        print(f"{path.relative_to(ROOT)}: üç dizin sütunlarıyla, anahtar tekrarı reddediyor")
    return 1 if problems else 0


def main() -> int:
    if "--files" in sys.argv[1:]:
        files()
        return 0
    if "--written" in sys.argv[1:]:
        return written()
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
