"""Writes the shared cases of a layer's record rules and its fields' new rules (docs/adr/0438) without KentOS code:

- fixtures/commands/v1/cad.layers.rules.json: the rules themselves (§1);
- fixtures/commands/v1/cad.entities.set.rules.json and cad.polygon.create.rules.json: what they, a unique field (§3) and
  a range's left-out end (§4) do to the records;
- fixtures/commands/v1/cad.forms.validate.rules.json: a form over the layer shows the rules on their fields, asks its
  checks at their moments (§2) and its listed values on the day the rules read (§5);
- fixtures/commands/v1/cad.forms.submit.rules.json: the form's release and the record as the screen opened them (§6, §7).

    python3 scripts/fixtures/layer_rules_cases.py           # writes the files
    python3 scripts/fixtures/layer_rules_cases.py --check   # compares them with the ones on disk

The rules are evaluated by the forms' twin (forms_cases.py, which evaluates the expression language without KentOS code);
a field's values by the layer fields' twin (layer_field_cases.py).
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

DURUM = {"name": "Durum", "kind": "text", "values": [
    {"code": "iyi", "label": "İyi", "description": "Taşıyıcı sistemde kusur yok."},
    {"code": "inceleme", "label": "Ek inceleme gerekli"},
    {"code": "eski", "label": "Eski sınıf", "until": "2025-12-31"},
    {"code": "yeni", "label": "Yeni sınıf", "from": "2027-01-01"}]}
FIELDS = [
    DURUM,
    {"id": "ada-parsel", "name": "Ada/Parsel", "kind": "text", "unique": True},
    {"name": "Başlangıç", "kind": "date"},
    {"name": "Bitiş", "kind": "date"},
    {"name": "Kat", "kind": "integer", "min": "0", "max": "60", "minExclusive": True},
    {"name": "Alan", "kind": "decimal", "scale": 2, "unit": "m²"},
    {"name": "Açıklama", "kind": "text"},
]
SIRA = {"id": "tarih-sirasi", "name": "Tarih sırası", "rule": "Bitiş boş veya Başlangıç boş veya Bitiş >= Başlangıç",
        "message": "Bitiş tarihi başlangıçtan önce olamaz.", "fields": ["Bitiş"]}
NOT = {"id": "inceleme-notu", "name": "İnceleme notu", "rule": "Durum <> 'inceleme' veya Açıklama boş değil",
       "message": "Ek inceleme gereken yapıya açıklama yazın.", "warning": True}
KAT = {"id": "kat-alan", "name": "Kat ve alan", "rule": "Alan boş veya Kat boş veya Alan <= Kat * 500",
       "message": "Kat başına en çok 500 m² olur."}
RULES = [SIRA, NOT, KAT]


def key(f):
    return f.get("id") or f["name"]


def layer(i, name, fields=None, kind="layer", children=None, **more):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields:
        out["fields"] = copy.deepcopy(fields)
    out.update(more)
    return out


def layers(rules=()):
    bina = layer("bina", "Ruhsatlı yapı", FIELDS)
    if rules:
        bina["rules"] = copy.deepcopy(list(rules))
    return [bina, layer("grup", "Koruma alanı", kind="group", children=[layer("alt", "Sit alanı")]),
            layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                                                      "maxZoom": 19})]


def square(x):
    return [{"x": E0 + x, "y": N0}, {"x": E0 + x + 10, "y": N0}, {"x": E0 + x + 10, "y": N0 + 10}, {"x": E0 + x, "y": N0 + 10}]


def setup(rules=(), entities=(), forms=None):
    settings = dict(SETTINGS)
    if forms:
        settings["forms"] = forms
    return {"format": "kentos.document", "version": 1, "name": "Kayıt kuralları", "settings": settings,
            "origin": {"x": E0, "y": N0}, "layers": layers(rules), "activeLayer": "bina", "entities": list(entities),
            "styles": {"items": [], "categories": []}}


class Refused(Exception):
    def __init__(self, code, message, path, status="failed"):
        super().__init__(message)
        self.error = {"code": code, "message": message, "path": path}
        self.status = status


# ---- The rules (§1) ----

def problem(rules):
    """The contract's `layer_rules_problem` over FIELDS, with its words (the expressions' are compiled after)."""
    keys = [key(f) for f in FIELDS]
    if len(rules) > 64:
        return "", f"Bir katmanın en çok 64 kayıt kuralı olur; {len(rules)} var."

    def bad(s, most):
        return not s.strip() or s.strip() != s or len(s) > most or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in s)
    for i, r in enumerate(rules):
        at = str(i)
        if bad(r["id"], 64):
            return f"{at}/id", f"“{r['id']}” bir kural kimliği olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan."
        if any(o["id"] == r["id"] for o in rules[:i]):
            return f"{at}/id", f"“{r['id']}” kuralı iki kez var."
        if bad(r["name"], 64):
            return f"{at}/name", f"“{r['id']}” kuralının adı 1–64 karakter olmalı, başında ve sonunda boşluk olmadan."
        if not r["rule"].strip() or len(r["rule"]) > 2000:
            return f"{at}/rule", f"“{r['name']}” kuralının ifadesi boş olamaz ve en çok 2000 karakterdir."
        if not r["message"].strip() or len(r["message"]) > 500:
            return f"{at}/message", f"“{r['name']}” kuralının iletisi boş olamaz ve en çok 500 karakterdir."
        for j, k in enumerate(r.get("fields", [])):
            if k not in keys:
                return f"{at}/fields/{j}", f"“{r['name']}” kuralının gösterildiği “{k}” alanı katmanda yok."
            if k in r["fields"][:j]:
                return f"{at}/fields/{j}", f"“{r['name']}” kuralında “{k}” alanı iki kez var."
    return None


def compile_problem(rules):
    """The first rule whose expression does not compile ($any: the compiler's words) or reads a field the layer lacks."""
    for i, r in enumerate(rules):
        tree = fc.parse(r["rule"])
        if tree is None:
            return i, ANY
        missing = next((n for n in fc.fields_read(tree) if not any(f["name"] == n for f in FIELDS)), None)
        if missing:
            return i, f"“{r['name']}” kuralı katmanda olmayan “{missing}” alanını okuyor."
    return None


def fit(rules, before, after, at="attrs"):
    """A write against the rules: a broken hard rule refuses `rule_broken` at its field, a soft one warns."""
    out = []
    for r, f in fc.rules_broken(rules, FIELDS, before, after, {"variables": {}, "object": {}}):
        path = at if f is None else f"{at}.{f['name']}"
        words = f"“{r['name']}” kuralı: {r['message']}"
        if not r.get("warning"):
            raise Refused("rule_broken", words, path)
        out.append({"code": "rule_unmet", "message": words, "path": path})
    return out


def unique(writes, records):
    """§3: writes (slot or None, before, after, at) against the unique fields; `records` every record now by slot."""
    for f in FIELDS:
        if not f.get("unique"):
            continue
        n = f["name"]

        def fresh(w):
            v = w[2].get(n)
            if v is None or not v.strip():
                return None
            return None if w[1] is not None and w[1].get(n) == v else v
        if not any(fresh(w) for w in writes):
            continue
        counts = {}
        written = {w[0] for w in writes if w[0] is not None}
        for slot, attrs in records.items():
            if slot not in written and attrs.get(n) is not None:
                counts[attrs[n]] = counts.get(attrs[n], 0) + 1
        for w in writes:
            v = w[2].get(n)
            if v is not None and v.strip():
                counts[v] = counts.get(v, 0) + 1
        for w in writes:
            v = fresh(w)
            if v is not None and counts.get(v, 0) > 1:
                raise Refused("not_unique", f"“{f['name']}” alanı katmanda tekil; “{v}” başka bir kayıtta var. Başka bir değer "
                                            "verin.", f"{w[3]}.{n}")


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


def tree(rules):
    return layers(rules)


def rules_file():
    cases = []
    base = setup()

    def ok(inp, now=None, undo=False):
        assert problem(inp.get("rules", [])) is None and compile_problem(inp.get("rules", [])) is None
        changed = (now or []) != inp.get("rules", [])
        expect = {"canUndo": changed or undo, "canRedo": False, "revision": "changed" if changed else "same",
                  "layers": tree(inp.get("rules", []))}
        if changed:
            expect["dirty"] = True
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "layer": inp["layer"], "changed": changed, "revision": "$current"}, "warnings": []}, "expect": expect}

    def refused(name, inp, c, message, path, status="failed"):
        err = {"code": c, "message": message, "path": path}
        cases.append({"name": name, "setup": base, "steps": [
            {"op": "execute", "input": inp, "result": {"status": status, "error": err},
             "expect": {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "layers": tree([])}}]})

    def bad(name, rules, path, message):
        assert problem(rules) == (path, message), (problem(rules), path, message)
        refused(name, {"layer": "bina", "rules": rules}, "invalid_rules", message, "rules" if not path else f"rules/{path}")

    def uncompiled(name, rules, at, message):
        assert problem(rules) is None and compile_problem(rules) == (at, message), compile_problem(rules)
        refused(name, {"layer": "bina", "rules": rules}, "invalid_rules", message, f"rules/{at}/rule")

    one = ok({"layer": "bina", "rules": RULES})
    cases.append({"name": "üç kural yazılır; adımı “Kayıt kuralları”, geri alınır ve yinelenir", "setup": base, "steps": [
        one, {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": tree([])}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": tree(RULES)}}]})
    cases.append({"name": "aynı kurallar yeniden: değişmez", "setup": base, "steps": [
        one, ok({"layer": "bina", "rules": RULES}, now=RULES, undo=True)]})
    cases.append({"name": "boş liste kuralları kaldırır", "setup": base, "steps": [one, ok({"layer": "bina"}, now=RULES, undo=True)]})
    cases.append({"name": "kuralsız katmana boş liste: değişmez", "setup": base, "steps": [ok({"layer": "bina"})]})
    cases.append({"name": "yalnız uyarı kuralı", "setup": base, "steps": [ok({"layer": "bina", "rules": [NOT]})]})
    cases.append({"name": "alanı verilmemiş kural (okuduğu ilk alanda gösterilir)", "setup": base,
                  "steps": [ok({"layer": "bina", "rules": [KAT]})]})
    cases.append({"name": "geometri değeri okuyan kural", "setup": base, "steps": [ok({"layer": "bina", "rules": [
        {"id": "en-az", "name": "En küçük yapı", "rule": "10 <= $alan", "message": "Yapı en az 10 m² olur."}]})]})
    node = copy.deepcopy(layers()[0])
    node["rules"] = [SIRA]
    cases.append({"name": "plan katmanı yazılacak hâliyle gösterir; doğrulama", "setup": base, "steps": [
        {"op": "plan", "input": {"layer": "bina", "rules": [SIRA]}, "result": {"status": "completed", "output": {
            "node": node, "changed": True, "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "dirty": False, "revision": "same", "layers": tree([])}},
        {"op": "validate", "input": {"layer": "bina", "rules": [SIRA]},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same"}}]})
    first = ok({"layer": "bina", "rules": [NOT]})
    cases.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", "setup": base,
                  "steps": [{"op": "captureRevision", "as": "once"}, first, {
                      "op": "execute", "input": {"layer": "bina", "rules": RULES, "expectedRevision": "$once"},
                      "result": {"status": "conflict", "error": {
                          "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                          "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                          "revision": "$current"}},
                      "expect": {"revision": "same", "layers": tree([NOT])}}]})
    refused("geçersiz beklenen sürüm", {"layer": "bina", "rules": RULES, "expectedRevision": "x"}, "invalid_revision",
            "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.", "expectedRevision")
    refused("çizimde olmayan katman", {"layer": "yok", "rules": RULES}, "layer_not_found",
            "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")
    refused("grup", {"layer": "grup", "rules": RULES}, "not_a_layer",
            "“Koruma alanı” bir katman grubu; kayıt kuralları yalnız katmanın olur. Grubun bir katmanını verin.", "layer")
    refused("servis katmanı", {"layer": "altlik", "rules": RULES}, "service_layer",
            "“OpenStreetMap” servisten çizilir ve nesne tutmaz; kayıt kuralı yoktur.", "layer")
    r = lambda **k: {**copy.deepcopy(SIRA), **k}  # noqa: E731
    bad("boşluklu kimlik", [r(id=" sira")], "0/id",
        "“ sira” bir kural kimliği olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.")
    bad("iki kez aynı kimlik", [SIRA, r(name="Başka")], "1/id", "“tarih-sirasi” kuralı iki kez var.")
    bad("boş ad", [r(name="")], "0/name", "“tarih-sirasi” kuralının adı 1–64 karakter olmalı, başında ve sonunda boşluk olmadan.")
    bad("boş ifade", [r(rule="  ")], "0/rule", "“Tarih sırası” kuralının ifadesi boş olamaz ve en çok 2000 karakterdir.")
    bad("boş ileti", [r(message="")], "0/message", "“Tarih sırası” kuralının iletisi boş olamaz ve en çok 500 karakterdir.")
    bad("uzun ileti", [r(message="x" * 501)], "0/message",
        "“Tarih sırası” kuralının iletisi boş olamaz ve en çok 500 karakterdir.")
    bad("olmayan alanda gösterilen kural", [r(fields=["Renk"])], "0/fields/0",
        "“Tarih sırası” kuralının gösterildiği “Renk” alanı katmanda yok.")
    bad("aynı alan iki kez", [r(fields=["Bitiş", "Bitiş"])], "0/fields/1", "“Tarih sırası” kuralında “Bitiş” alanı iki kez var.")
    many = [r(id=f"k{i}", name=f"Kural {i}") for i in range(65)]
    bad("65 kural", many, "", "Bir katmanın en çok 64 kayıt kuralı olur; 65 var.")
    uncompiled("derlenmeyen ifade", [r(rule="Bitiş >= ")], 0, ANY)
    uncompiled("katmanda olmayan alanı okuyan ifade", [SIRA, {**copy.deepcopy(KAT), "rule": "Taban <= Kat * 500"}], 1,
               "“Kat ve alan” kuralı katmanda olmayan “Taban” alanını okuyor.")
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.rules", "commandVersion": 1,
            "title": "Kayıt kuralları",
            "note": "ADR 0438 §1: katmanın kayıt kuralları ve kuralları. Written by scripts/fixtures/layer_rules_cases.py "
                    "without KentOS code; do not edit by hand.", "setup": base, "cases": cases}


# ---- cad.entities.set ----

RECORDS = {
    1: {"Durum": "iyi", "Ada/Parsel": "101/5", "Başlangıç": "2026-03-01", "Bitiş": "2026-09-30", "Kat": "3", "Alan": "900.00"},
    2: {"Durum": "inceleme", "Ada/Parsel": "101/6", "Açıklama": "Çatı arası.", "Kat": "2", "Alan": "400.00"},
    3: {},
    # Already breaks the floors' rule: a write that does not touch its fields does not ask it.
    4: {"Kat": "1", "Alan": "700.00"},
}
ENTITIES = [{"kind": "polygon", "id": i, "layerId": "bina", "attrs": a, "pts": square(20 * i)} for i, a in RECORDS.items()]


def set_file():
    cases = []

    def step(uids, attrs, now=None, undo=False):
        now = copy.deepcopy(now or RECORDS)
        after_all = copy.deepcopy(now)
        warnings = []
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
                for w in fit(RULES, now[u], after):
                    if not any(x["code"] == w["code"] and x["path"] == w["path"] for x in warnings):
                        warnings.append(w)
                writes.append((u, now[u], after, "attrs"))
                after_all[u] = after
            unique(writes, now)
        except Refused as r:
            return {"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error},
                    "expect": {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in now.items()},
                               "canUndo": undo, "canRedo": False, "revision": "same",
                               **({} if undo else {"dirty": False})}}, now
        changed = [u for u in uids if after_all[u] != now[u]]
        expect = {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in after_all.items()},
                  "canUndo": bool(changed) or undo, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "changed": [f"$uidOf:{u}" for u in changed], "ids": changed, "revision": "$current"}, "warnings": warnings},
            "expect": expect}, after_all

    def case(name, uids, attrs):
        s, _ = step(uids, attrs)
        cases.append({"name": name, "steps": [s]})

    case("bitiş başlangıçtan önce: reddedilir, yeri Bitiş", [1], {"Bitiş": "2026-02-01"})
    case("başlangıç bitişten sonraya: reddedilir, yeri kuralın alanı Bitiş", [1], {"Başlangıç": "2026-10-01"})
    case("tarihler sıralı", [1], {"Başlangıç": "2026-01-15", "Bitiş": "2026-12-31"})
    case("bitiş boşalır: kural tutar (boş yazılmış)", [1], {"Bitiş": None})
    case("yalnız başlangıçlı kayıt", [3], {"Başlangıç": "2026-05-05"})
    case("inceleme açıklamasız: uyarılır, yazılır", [1], {"Durum": "inceleme"})
    case("incelemenin açıklaması silinir: uyarılır", [2], {"Açıklama": None})
    case("inceleme açıklamasıyla: uyarı yok", [1], {"Durum": "inceleme", "Açıklama": "Kolonlarda çatlak."})
    case("alan kat başına sınırı aşar: yeri okuduğu ilk alan Alan", [1], {"Alan": "1600"})
    case("kat azalınca alan sınırı aşar", [1], {"Kat": "1"})
    case("kat ve alan birlikte geçerli", [1], {"Kat": "4", "Alan": "1950.5"})
    case("kuralı bozuk kayıtta kuralın alanlarına dokunmayan yazma sorulmaz", [4], {"Durum": "iyi"})
    case("kuralı bozuk kayıtta alanı değişen kural sorulur", [4], {"Alan": "650"})
    case("kat 0 aralığın hariç ucu: reddedilir", [1], {"Kat": "0"})
    case("kat 60 aralığın içinde", [1], {"Kat": "60"})
    case("ada/parsel başka kayıtta var: tekil", [3], {"Ada/Parsel": "101/5"})
    case("iki kayda aynı ada/parsel birden: tekil", [3, 4], {"Ada/Parsel": "102/1"})
    case("kendi ada/parselini yeniden yazmak serbest", [1], {"Ada/Parsel": "101/5"})
    case("ada/parsel boşaltmak serbest", [1, 2], {"Ada/Parsel": None})
    case("iki kayıt birden: biri kuralı bozarsa hiçbiri yazılmaz", [1, 2], {"Bitiş": "2026-04-01", "Başlangıç": "2026-06-01"})
    case("iki kaydın uyarısı tek", [1, 3], {"Durum": "inceleme"})
    one, after = step([1], {"Durum": "inceleme"})
    two, _ = step([1], {"Açıklama": "Bodrumda nem."}, now=after, undo=True)
    cases.append({"name": "uyarıdan sonra açıklama yazılır", "steps": [one, two]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.set", "commandVersion": 1,
            "title": "Öznitelikler ve kayıt kuralları",
            "note": "ADR 0438 §1, §3, §4: okuduğu alanı değişen kural sorulur; kural bozulursa reddedilir, uyarı kuralı söylenir; "
                    "tekil alanın değeri katmanda bir kez; aralığın hariç ucu dışarıdadır. Written by "
                    "scripts/fixtures/layer_rules_cases.py without KentOS code.",
            "setup": setup(RULES, ENTITIES), "cases": cases}


# ---- cad.polygon.create ----

def create_file():
    cases = []
    existing = {1: RECORDS[1]}

    def made(name, attrs=None):
        inp = {"layerId": "bina", "pts": square(200)}
        try:
            out = {}
            if attrs:
                inp["attrs"] = attrs
                out = {k: v for k, v in values_of(attrs, "attrs").items() if v}
            fit(RULES, None, out)
            unique([(None, None, out, "attrs")], existing)
        except Refused as r:
            cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error},
                                                   "expect": {"ids": [1], "canUndo": False, "dirty": False,
                                                              "revision": "same"}}]})
            return
        expect = {"ids": [1, 2], "canUndo": True, "dirty": True, "revision": "changed"}
        if out:
            expect["fields"] = {"2": {"attrs": out}}
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": {"uid": "$uid", "id": 2, "revision": "$current"}, "warnings": []},
            "expect": expect}]})

    made("değersiz kayıt: kurallar boş cevapla tutar", None)
    made("tarihleri sıralı yeni kayıt", {"Başlangıç": "2026-01-01", "Bitiş": "2026-06-30"})
    made("bitişi başlangıçtan önce yeni kayıt reddedilir", {"Başlangıç": "2026-06-01", "Bitiş": "2026-01-01"})
    made("aynı günde başlayıp biten", {"Başlangıç": "2026-06-01", "Bitiş": "01.06.2026"})
    made("yalnız bitiş", {"Bitiş": "2026-06-01"})
    made("inceleme açıklamasız: çizim komutu uyarı kuralını söylemez, yazar", {"Durum": "inceleme"})
    made("alan sınırında", {"Kat": "2", "Alan": "1000"})
    made("alan sınırı aşar", {"Kat": "2", "Alan": "1000.01"})
    made("katsız alan", {"Alan": "5000"})
    made("kat 0 reddedilir: hariç uç", {"Kat": "0"})
    made("kat 1", {"Kat": "1"})
    made("kat 60", {"Kat": "60"})
    made("kat 61 reddedilir", {"Kat": "61"})
    made("ada/parsel başka kayıtta var", {"Ada/Parsel": "101/5"})
    made("yeni ada/parsel", {"Ada/Parsel": "101/7"})
    made("boşluklu ada/parsel aynı değil", {"Ada/Parsel": "101/5 "})
    made("durum etiketiyle", {"Durum": "İyi"})
    made("bütün alanlar geçerli", {"Durum": "inceleme", "Ada/Parsel": "103/2", "Başlangıç": "2026-02-01",
                                   "Bitiş": "2026-03-01", "Kat": "5", "Alan": "2400.25", "Açıklama": "Ön inceleme."})
    made("ondalık alan virgülle", {"Kat": "1", "Alan": "499,99"})
    made("alan sınırı virgüllü değerle aşılır", {"Kat": "1", "Alan": "500,01"})
    made("tarih GG.AA.YYYY'den ve sıra bozuk", {"Başlangıç": "05.06.2026", "Bitiş": "04.06.2026"})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.polygon.create", "commandVersion": 1,
            "title": "Alan çiz ve kayıt kuralları",
            "note": "ADR 0438 §1, §3, §4: yeni kayıt kuralları bozamaz, tekil değeri yineleyemez; çizim komutlarında uyarı kuralı "
                    "söylenmez. Written by scripts/fixtures/layer_rules_cases.py without KentOS code.",
            "setup": setup(RULES, [ENTITIES[0]]), "cases": cases}


# ---- cad.forms.validate ----

BODY = {"revision": 1, "items": [
    {"type": "field", "id": "durum", "field": "Durum"},
    {"type": "field", "id": "ada", "field": "ada-parsel"},
    {"type": "field", "id": "baslangic", "field": "Başlangıç"},
    {"type": "field", "id": "bitis", "field": "Bitiş"},
    {"type": "field", "id": "kat", "field": "Kat", "checks": [
        {"rule": "Kat <= 40", "message": "Bu yapı türünde en çok 40 kat olur.", "on": "submit"}]},
    {"type": "field", "id": "alan", "field": "Alan"},
    {"type": "field", "id": "aciklama", "field": "Açıklama", "checks": [
        {"rule": "uzunluk(Açıklama) >= 5", "message": "Açıklama en az 5 karakter olmalı.", "on": "save"}]},
]}
FLOW = {**copy.deepcopy(BODY), "workflow": {"field": "Durum", "initial": "iyi", "steps": [
    {"id": "incele", "label": "İncelemeye gönder", "from": ["iyi"], "to": "inceleme", "valid": True}]}}
FORM = {"id": "ruhsat", "name": "Ruhsat", "layer": "bina", "draft": BODY,
        "releases": [{"version": 1, "digest": fc.digest(BODY), "body": BODY}]}
GIVEN = [{"name": "tarih", "value": "2026-10-16"}, {"name": "simdi", "value": "2026-10-16T14:30:00"}]
MOMENT = "2026-10-16T14:30:00"


def validate_file():
    cases = []
    S = setup(RULES, ENTITIES, [FORM])

    def run(name, values=None, slot=3, defaults=False, moment=False, body=BODY):
        variables = GIVEN if moment else ()
        f = fc.Form(body, FIELDS, variables, rules=RULES)
        obj = {"length": 40, "area": 100, "vertices": 4}
        after, found = f.assess(dict(RECORDS[slot]), fc.utf8_sorted(dict(values or {})), obj, defaults)
        others = [a for s, a in RECORDS.items() if s != slot]
        issues = fc.preflight(body, FIELDS, variables) + found + f.unique_issues(after, obj, others)
        out = {"issues": fc.shown_issues(issues), "valid": not fc.errors(issues), "items": f.described(after, obj),
               "version": 1, "attrs": after}
        inp = {"form": "ruhsat", "uid": f"$uidOf:{slot}"}
        if values:
            inp["values"] = values
        if defaults:
            inp["defaults"] = True
        if moment:
            inp["moment"] = MOMENT
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp,
                                               "result": {"status": "completed", "output": out, "warnings": []},
                                               "expect": {"ids": list(RECORDS), "canUndo": False, "canRedo": False,
                                                          "dirty": False, "revision": "same"}}]})

    run("geçerli kayıt: sorun yok", slot=1)
    run("kural bozan değer kuralın alanında söylenir", {"Bitiş": "2026-02-01"}, slot=1)
    run("uyarı kuralı uyarı olarak", {"Durum": "inceleme"}, slot=1)
    run("kayıt zaten kuralı bozuyorsa form söyler (alanı verilmemiş kural okuduğu ilk alanda)", slot=4)
    run("kat ve alanı düzeltmek", {"Kat": "2"}, slot=4)
    run("tekil alan: başka kaydın değeri", {"ada-parsel": "101/6"}, slot=1)
    run("tekil alan: yeni değer", {"ada-parsel": "200/1"}, slot=1)
    run("kaydederken sorulan denetim kaydetmede sorulur", {"Açıklama": "kısa"}, slot=2)
    run("kaydederken sorulan denetim tutar", {"Açıklama": "Çatı arası nemli."}, slot=2)
    run("iş akışı yokken ilerletmede sorulan denetim kaydetmede sorulur", {"Kat": "45", "Alan": "1000"}, slot=1)
    run("hariç uç: kat 0 alanın kuralına uymaz", {"Kat": "0"}, slot=1)
    run("geçerlilik: tarih verilmeden yeni sınıf seçilir", {"Durum": "yeni"}, slot=3)
    run("geçerlilik: bugün henüz geçerli olmayan değer yazılamaz", {"Durum": "yeni"}, slot=3, moment=True)
    run("geçerlilik: süresi dolmuş değer yazılamaz", {"Durum": "eski"}, slot=3, moment=True)
    run("geçerlilik: geçerli değer", {"Durum": "iyi"}, slot=3, moment=True)
    run("yeni kayıt varsayılanlarla", defaults=True)
    run("boş kayıt", slot=3)
    run("kat 60 ve alan 30000", {"Kat": "60", "Alan": "30000"}, slot=1)
    run("tarihler birlikte düzeltilir", {"Başlangıç": "2026-01-01", "Bitiş": "2026-01-01"}, slot=1)
    run("açıklama boşaltılınca inceleme uyarısı", {"Açıklama": ""}, slot=2)
    run("etiketle durum", {"Durum": "Ek inceleme gerekli", "Açıklama": "Yeniden bakılacak."}, slot=1)
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.validate", "commandVersion": 1,
            "title": "Formu denetle ve kayıt kuralları",
            "note": "ADR 0438 §1–§5: form katmanın kurallarını alanlarında söyler, tekil alanı bilir, denetimleri zamanlarında "
                    "sorar, değerleri kuralların gününde sunar. Written by scripts/fixtures/layer_rules_cases.py with the form "
                    "twin of forms_cases.py, without KentOS code; the issues' sentences are the core's ($any).",
            "setup": S, "cases": cases}


# ---- cad.forms.submit ----

def submit_file():
    cases = []
    two = {**copy.deepcopy(FORM), "releases": FORM["releases"] + [{"version": 2, "digest": fc.digest(BODY), "body": BODY}]}
    S = setup(RULES, ENTITIES, [FORM])
    S2 = setup(RULES, ENTITIES, [two])

    def write(name, inp, slot, values, setup_=S, version=1):
        f = fc.Form(BODY, FIELDS, (), rules=RULES)
        obj = {"length": 40, "area": 100, "vertices": 4}
        before = dict(RECORDS[slot])
        after, issues = f.assess(before, fc.utf8_sorted(dict(values)), obj, False)
        others = [a for s, a in RECORDS.items() if s != slot]
        taken = f.unique_issues(after, obj, others)
        bad = fc.errors(issues) or taken
        if bad:
            first = bad[0]
            path = f"values.{first['field']}" if first.get("field") in values else "values"
            cases.append({"name": name, "setup": setup_, "steps": [{"op": "execute", "input": inp, "result": {
                "status": "failed", "error": {"code": "form_invalid", "message": ANY, "path": path}},
                "expect": {"ids": list(RECORDS), "fields": {str(slot): {"attrs": before}}, "canUndo": False,
                           "canRedo": False, "revision": "same", "dirty": False}}]})
            return
        warnings = [{"code": i["code"], "message": ANY, "path": f"values.{i['field']}"} for i in issues
                    if i["severity"] == "warning"]
        changed = after != before
        expect = {"ids": list(RECORDS), "fields": {str(slot): {"attrs": after}}, "canUndo": changed, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        cases.append({"name": name, "setup": setup_, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": {"uid": f"$uidOf:{slot}", "version": version, "attrs": after, "changed": changed,
                                              "revision": "$current"}, "warnings": warnings}, "expect": expect}]})

    def conflict(name, inp, slot, c, message, path, setup_=S):
        cases.append({"name": name, "setup": setup_, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "conflict", "error": {"code": c, "message": message, "path": path}},
            "expect": {"ids": list(RECORDS), "fields": {str(slot): {"attrs": RECORDS[slot]}}, "canUndo": False,
                       "canRedo": False, "revision": "same", "dirty": False}}]})

    v = {"Kat": "4"}
    write("açıldığı yayınla ve kaydın açıldığı hâliyle yazar", {"form": "ruhsat", "uid": "$uidOf:1", "values": v, "release": 1,
                                                               "opened": RECORDS[1]}, 1, v)
    conflict("form açıldıktan sonra yeniden yayınlandı: yazılmaz",
             {"form": "ruhsat", "uid": "$uidOf:1", "values": v, "release": 1}, 1, "form_release_changed",
             "“Ruhsat” formu 1. yayınla açıldı; şimdi 2. yayında. Formu yeniden açıp değerleri yeniden girin.", "release", S2)
    write("son yayınla açılan ekran yazar", {"form": "ruhsat", "uid": "$uidOf:1", "values": v, "release": 2}, 1, v, S2, 2)
    write("yayın verilmezse son yayın", {"form": "ruhsat", "uid": "$uidOf:1", "values": v}, 1, v, S2, 2)
    changed_opened = {**RECORDS[1], "Kat": "2"}
    conflict("kayıt form açıldıktan sonra değişti: üzerine yazılmaz",
             {"form": "ruhsat", "uid": "$uidOf:1", "values": v, "opened": changed_opened}, 1, "record_changed",
             "Kayıt form açıldıktan sonra değişti (Kat). Formu yeniden yükleyin ya da değerlerinizi yeniden kaydedin.", "opened")
    gone = {k: x for k, x in RECORDS[1].items() if k != "Alan"}
    conflict("açıldığında olmayan öznitelik eklenmiş: değişti sayılır",
             {"form": "ruhsat", "uid": "$uidOf:1", "values": v, "opened": gone}, 1, "record_changed",
             "Kayıt form açıldıktan sonra değişti (Alan). Formu yeniden yükleyin ya da değerlerinizi yeniden kaydedin.", "opened")
    write("kural bozan değer: form_invalid, yeri değerin", {"form": "ruhsat", "uid": "$uidOf:1",
                                                            "values": {"Bitiş": "2026-01-01"}}, 1, {"Bitiş": "2026-01-01"})
    write("uyarı kuralı uyarıyla yazılır", {"form": "ruhsat", "uid": "$uidOf:1", "values": {"Durum": "inceleme"}}, 1,
          {"Durum": "inceleme"})
    write("tekil alan başka kayıtta var: form_invalid", {"form": "ruhsat", "uid": "$uidOf:3",
                                                         "values": {"ada-parsel": "101/5"}}, 3, {"ada-parsel": "101/5"})
    write("kaydederken sorulan denetim bozulur: form_invalid", {"form": "ruhsat", "uid": "$uidOf:2",
                                                                "values": {"Açıklama": "kısa"}}, 2, {"Açıklama": "kısa"})
    write("ilerletmede sorulan denetim iş akışsız formda kaydetmede sorulur", {"form": "ruhsat", "uid": "$uidOf:1",
                                                                               "values": {"Kat": "45", "Alan": "10"}}, 1,
          {"Kat": "45", "Alan": "10"})
    write("açıldığı hâliyle değişmemiş kayıt ve aynı değer: değişmez", {"form": "ruhsat", "uid": "$uidOf:1",
                                                                        "values": {"Kat": "3"}, "opened": RECORDS[1]}, 1,
          {"Kat": "3"})
    write("boş kayda açıldığı hâliyle (boş) yazılır", {"form": "ruhsat", "uid": "$uidOf:3", "values": {"Kat": "2"},
                                                      "opened": {}}, 3, {"Kat": "2"})
    conflict("boş açılan kayda başkası yazmış", {"form": "ruhsat", "uid": "$uidOf:4", "values": {"Kat": "2"}, "opened": {}}, 4,
             "record_changed", "Kayıt form açıldıktan sonra değişti (Alan, Kat). Formu yeniden yükleyin ya da değerlerinizi "
             "yeniden kaydedin.", "opened")
    write("tarihler birlikte geçerli", {"form": "ruhsat", "uid": "$uidOf:1",
                                        "values": {"Başlangıç": "2026-01-01", "Bitiş": "2026-12-31"}}, 1,
          {"Başlangıç": "2026-01-01", "Bitiş": "2026-12-31"})
    write("alan kat başına sınırı aşar: form_invalid", {"form": "ruhsat", "uid": "$uidOf:1", "values": {"Alan": "1600"}}, 1,
          {"Alan": "1600"})
    write("bozuk kaydı düzelten yazma", {"form": "ruhsat", "uid": "$uidOf:4", "values": {"Kat": "2"}}, 4, {"Kat": "2"})
    write("hariç uç: kat 0", {"form": "ruhsat", "uid": "$uidOf:1", "values": {"Kat": "0"}}, 1, {"Kat": "0"})
    write("açıklamayla inceleme: uyarısız", {"form": "ruhsat", "uid": "$uidOf:1",
                                             "values": {"Durum": "inceleme", "Açıklama": "Kolonlar."}}, 1,
          {"Durum": "inceleme", "Açıklama": "Kolonlar."})
    write("tekil alana yeni değer", {"form": "ruhsat", "uid": "$uidOf:3", "values": {"ada-parsel": "300/4"}}, 3,
          {"ada-parsel": "300/4"})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.submit", "commandVersion": 1,
            "title": "Formla kaydet: yayın, açılan kayıt ve kayıt kuralları",
            "note": "ADR 0438 §1–§3, §6, §7: ekranın açıldığı yayın ve kaydın açıldığı hâli; kurallar ve tekil alan formdan. "
                    "Written by scripts/fixtures/layer_rules_cases.py with the form twin of forms_cases.py, without KentOS "
                    "code.", "setup": S, "cases": cases}


def outputs():
    return {OUT / "cad.layers.rules.json": rules_file(), OUT / "cad.entities.set.rules.json": set_file(),
            OUT / "cad.polygon.create.rules.json": create_file(), OUT / "cad.forms.validate.rules.json": validate_file(),
            OUT / "cad.forms.submit.rules.json": submit_file()}


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
