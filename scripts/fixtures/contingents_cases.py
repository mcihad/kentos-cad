"""Writes the shared cases of a layer's value groups (Değer bileşimleri, docs/adr/0423) without KentOS code: the rule of
the groups (§1) by `cad.layers.contingents` v1 (§3), and what they do to the records (§2) by `cad.entities.set`,
`cad.polygon.create` and `cad.forms.validate`:

- fixtures/commands/v1/cad.layers.contingents.json
- fixtures/commands/v1/cad.entities.set.contingents.json
- fixtures/commands/v1/cad.polygon.create.contingents.json
- fixtures/commands/v1/cad.forms.validate.contingents.json

    python3 scripts/fixtures/contingents_cases.py           # writes the files
    python3 scripts/fixtures/contingents_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import subtypes_cases as st  # noqa: E402  (the subtypes' rule, written without KentOS code)

OUT = ROOT / "fixtures/commands/v1"
CATI = {"name": "Çatı", "kind": "text", "values": [st.choice("duz", "Düz"), st.choice("kirma", "Kırma"),
                                                   st.choice("teras", "Teras")]}
FIELDS = copy.deepcopy(st.FIELDS) + [CATI]


def code(c):
    return {"kind": "code", "code": c}


ANY, EMPTY = {"kind": "any"}, {"kind": "empty"}
YAPI = {"name": "Yapı", "fields": ["Tür", "Malzeme"], "restrictive": True, "rows": [
    {"cells": [code("konut"), ANY]},
    {"cells": [code("sanayi"), code("beton")]},
    {"cells": [code("sanayi"), code("celik")]},
    {"cells": [code("depo"), code("tas")]},
    {"cells": [code("depo"), EMPTY]},
]}
CATI_GROUP = {"name": "Çatı", "fields": ["Malzeme", "Çatı"], "rows": [
    {"cells": [code("ahsap"), code("kirma")]},
    {"cells": [code("beton"), ANY]},
    {"cells": [code("celik"), code("duz")]},
    {"cells": [code("celik"), code("teras")], "subtype": "sanayi"},
]}
GROUPS = [YAPI, CATI_GROUP]
SUBTYPES = {"field": "Tür", "types": [{"code": "konut"}, {"code": "sanayi"}, {"code": "depo"}]}


def layers(with_groups=True):
    out = st.layers(False)
    for n in out:
        if n["id"] in ("bina", "fabrika"):
            n["fields"] = copy.deepcopy(FIELDS)
    out[0]["subtypes"] = copy.deepcopy(SUBTYPES)
    if with_groups:
        out[0]["contingents"] = copy.deepcopy(GROUPS)
    return out


def setup(with_groups=True, entities=()):
    s = st.setup(False, entities)
    s["layers"] = layers(with_groups)
    s["name"] = "Değer bileşimleri"
    return s


# ---- The rule (§1, §2), written from the ADR ----

def key(f):
    return f.get("id") or f["name"]


def label(f):
    return f.get("alias") or f["name"]


def allows(cell, v):
    if cell == ANY:
        return True
    if cell == EMPTY:
        return v == ""
    return cell["code"] == v


def rows_for(g, subtype):
    return [r for r in g["rows"] if "subtype" not in r or r["subtype"] == subtype]


def values_of(g, fields, attrs):
    out = []
    for k in g["fields"]:
        f = next(f for f in fields if key(f) == k)
        out.append(attrs.get(f["name"], "").strip())
    return out


def holds(g, values, subtype):
    if all(v == "" for v in values):
        return True
    return any(all(allows(c, v) for c, v in zip(r["cells"], values)) for r in rows_for(g, subtype))


def offered(g, at, values, subtype):
    out = []
    for r in rows_for(g, subtype):
        if not all(i == at or v == "" or allows(c, v) for i, (c, v) in enumerate(zip(r["cells"], values))):
            continue
        c = r["cells"][at]
        if c == ANY:
            return None
        if c != EMPTY and c["code"] not in out:
            out.append(c["code"])
    return out


def words(g, fields, values):
    out = []
    for k, v in zip(g["fields"], values):
        f = next(f for f in fields if key(f) == k)
        shown = next((c["label"] for c in f.get("values", []) if c["code"] == v), v) if v else "boş"
        out.append(f"{label(f)} = {shown}")
    return ", ".join(out)


def subtype_of(attrs):
    v = attrs.get("Tür", "").strip()
    return v if v in ("konut", "sanayi", "depo") else None


def problem(fields, subtypes, groups):
    if len(groups) > 50:
        return "", f"Bir katmanın en çok 50 değer grubu olur; {len(groups)} var."
    for i, g in enumerate(groups):
        at = str(i)
        n = g["name"]
        if not n or len(n) > 64 or n.strip() != n:
            return f"{at}/name", f"“{n}” bir grup adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan."
        if any(o["name"] == n for o in groups[:i]):
            return f"{at}/name", f"“{n}” grubu iki kez var."
        if not 2 <= len(g["fields"]) <= 8:
            return f"{at}/fields", f"“{n}” grubunda 2–8 alan olur; {len(g['fields'])} var."
        for j, k in enumerate(g["fields"]):
            f = next((x for x in fields if key(x) == k), None)
            if f is None:
                return f"{at}/fields/{j}", f"Katmanda “{k}” alanı yok."
            if not f.get("values") or st.holds_many(f):
                return f"{at}/fields/{j}", f"“{label(f)}” alanı gruba giremez: değer listeli, tek değerli bir alan olmalı."
            if k in g["fields"][:j]:
                return f"{at}/fields/{j}", f"“{label(f)}” alanı “{n}” grubunda iki kez var."
        if not g["rows"]:
            return f"{at}/rows", f"“{n}” grubunda bileşim yok; en az bir bileşim verin."
        for r, row in enumerate(g["rows"]):
            at2 = f"{at}/rows/{r}"
            if len(row["cells"]) != len(g["fields"]):
                return f"{at2}/cells", f"{r + 1}. bileşimde {len(row['cells'])} hücre var; grubun {len(g['fields'])} alanı var."
            for c, (cell, k) in enumerate(zip(row["cells"], g["fields"])):
                if cell["kind"] == "code":
                    f = next(x for x in fields if key(x) == k)
                    if cell["code"] not in [v["code"] for v in f["values"]]:
                        return f"{at2}/cells/{c}", f"“{cell['code']}”, “{label(f)}” alanının listesinde yok."
            if "subtype" in row and not (subtypes and any(t["code"] == row["subtype"] for t in subtypes["types"])):
                return f"{at2}/subtype", f"{r + 1}. bileşimin alt türü “{row['subtype']}” katmanın alt türlerinden değil."
            if row in g["rows"][:r]:
                return at2, f"{r + 1}. bileşim daha önce var."
    return None


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.error = {"code": code, "message": message, "path": path}


def fit(fields, groups, before, after, at="attrs"):
    """The record against the value groups (§2): a touched group none of whose rows allows its values, refused when
    restrictive, warned when advisory."""
    out = []
    subtype = subtype_of(after)
    for g in groups:
        values = values_of(g, fields, after)
        if holds(g, values, subtype):
            continue
        first = None
        for k in g["fields"]:
            f = next(x for x in fields if key(x) == k)
            now, was = after.get(f["name"]), (before or {}).get(f["name"])
            if now != was and (before is not None or now is not None):
                first = f
                break
        if first is None:
            continue
        w = words(g, fields, values)
        if g.get("restrictive"):
            raise Refused("invalid_combination", f"“{g['name']}” değer grubunda bu bileşim yok: {w}. Grubun bileşimlerinden "
                                                 "birini seçin.", f"{at}.{first['name']}")
        out.append({"code": "combination_unmet", "message": f"“{g['name']}” değer grubunda bu bileşim yok: {w}.",
                    "path": f"{at}.{first['name']}"})
    return out


# ---- cad.layers.contingents ----

def tree(groups):
    out = layers(False)
    if groups:
        out[0]["contingents"] = groups
    return out


def contingents_file():
    cases = []
    base = setup(False)

    def ok(inp, now=None, undo=False):
        assert problem(FIELDS, SUBTYPES, inp.get("groups", [])) is None
        changed = (now or []) != inp.get("groups", [])
        expect = {"canUndo": changed or undo, "canRedo": False, "revision": "changed" if changed else "same",
                  "layers": tree(inp.get("groups", []))}
        if changed:
            expect["dirty"] = True
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "layer": inp["layer"], "changed": changed, "revision": "$current"}, "warnings": []}, "expect": expect}

    def refused(name, inp, c, message, path, status="failed"):
        err = {"code": c, "message": message, "path": path}
        if c == "revision_conflict":
            err["retryable"] = True
        cases.append({"name": name, "setup": base, "steps": [
            {"op": "execute", "input": inp, "result": {"status": status, "error": err},
             "expect": {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "layers": tree([])}}]})

    def bad(name, groups, path, message):
        assert problem(FIELDS, SUBTYPES, groups) == (path, message), (problem(FIELDS, SUBTYPES, groups), path)
        refused(name, {"layer": "bina", "groups": groups}, "invalid_contingents", message,
                "groups" if not path else f"groups/{path}")

    one = ok({"layer": "bina", "groups": GROUPS})
    cases.append({"name": "iki grup yazılır; adımı “Değer bileşimleri”, geri alınır ve yinelenir", "setup": base, "steps": [
        one, {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": tree([])}},
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": tree(GROUPS)}}]})
    cases.append({"name": "aynı gruplar yeniden: değişmez", "setup": base, "steps": [
        one, ok({"layer": "bina", "groups": GROUPS}, now=GROUPS, undo=True)]})
    cases.append({"name": "boş liste grupları kaldırır", "setup": base, "steps": [
        one, ok({"layer": "bina"}, now=GROUPS, undo=True)]})
    cases.append({"name": "grupsuz katmana boş liste: değişmez", "setup": base, "steps": [ok({"layer": "bina"})]})
    cases.append({"name": "tek danışma grubu", "setup": base, "steps": [ok({"layer": "bina", "groups": [CATI_GROUP]})]})
    node = copy.deepcopy(layers(False)[0])
    node["contingents"] = [YAPI]
    cases.append({"name": "plan katmanı yazılacak hâliyle gösterir; doğrulama", "setup": base, "steps": [
        {"op": "plan", "input": {"layer": "bina", "groups": [YAPI]}, "result": {"status": "completed", "output": {
            "node": node, "changed": True, "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "dirty": False, "revision": "same", "layers": tree([])}},
        {"op": "validate", "input": {"layer": "bina", "groups": [YAPI]},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same"}}]})
    first = ok({"layer": "bina", "groups": [CATI_GROUP]})
    cases.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", "setup": base,
                  "steps": [{"op": "captureRevision", "as": "once"}, first, {
                      "op": "execute", "input": {"layer": "bina", "groups": GROUPS, "expectedRevision": "$once"},
                      "result": {"status": "conflict", "error": {
                          "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                          "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                          "revision": "$current"}},
                      "expect": {"revision": "same", "layers": tree([CATI_GROUP])}}]})
    refused("geçersiz beklenen sürüm", {"layer": "bina", "groups": GROUPS, "expectedRevision": "x"}, "invalid_revision",
            "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.", "expectedRevision")
    refused("çizimde olmayan katman", {"layer": "yok", "groups": GROUPS}, "layer_not_found",
            "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")
    refused("grup", {"layer": "koruma", "groups": GROUPS}, "not_a_layer",
            "“Koruma alanı” bir katman grubu; değer bileşimleri yalnız katmanın olur. Grubun bir katmanını verin.", "layer")
    refused("servis katmanı", {"layer": "altlik", "groups": GROUPS}, "service_layer",
            "“OpenStreetMap” servisten çizilir ve nesne tutmaz; değer grubu yoktur.", "layer")
    g = lambda **k: {**copy.deepcopy(YAPI), **k}  # noqa: E731
    bad("boşluklu ad", [g(name=" Yapı")], "0/name", "“ Yapı” bir grup adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.")
    bad("iki kez aynı ad", [YAPI, g()], "1/name", "“Yapı” grubu iki kez var.")
    bad("tek alanlı grup", [g(fields=["Tür"], rows=[{"cells": [code("konut")]}])], "0/fields", "“Yapı” grubunda 2–8 alan olur; 1 var.")
    bad("olmayan alan", [g(fields=["Tür", "Renk"])], "0/fields/1", "Katmanda “Renk” alanı yok.")
    bad("listesiz alan", [g(fields=["Tür", "Not"])], "0/fields/1",
        "“Not” alanı gruba giremez: değer listeli, tek değerli bir alan olmalı.")
    bad("çok değerli alan", [g(fields=["Tür", "Etiket"])], "0/fields/1",
        "“Etiket” alanı gruba giremez: değer listeli, tek değerli bir alan olmalı.")
    bad("aynı alan iki kez", [g(fields=["Tür", "Tür"])], "0/fields/1", "“Tür” alanı “Yapı” grubunda iki kez var.")
    bad("bileşimsiz grup", [g(rows=[])], "0/rows", "“Yapı” grubunda bileşim yok; en az bir bileşim verin.")
    bad("eksik hücre", [g(rows=[{"cells": [code("konut")]}])], "0/rows/0/cells", "1. bileşimde 1 hücre var; grubun 2 alanı var.")
    bad("listede olmayan kod", [g(rows=[{"cells": [code("konut"), code("cam")]}])], "0/rows/0/cells/1",
        "“cam”, “Malzeme” alanının listesinde yok.")
    bad("olmayan alt tür", [g(rows=[{"cells": [code("konut"), ANY], "subtype": "ofis"}])], "0/rows/0/subtype",
        "1. bileşimin alt türü “ofis” katmanın alt türlerinden değil.")
    bad("aynı bileşim iki kez", [g(rows=[{"cells": [code("konut"), ANY]}, {"cells": [code("konut"), ANY]}])], "0/rows/1",
        "2. bileşim daha önce var.")
    bad("ikinci grubun sorunu yeriyle", [YAPI, {**copy.deepcopy(CATI_GROUP), "rows": []}], "1/rows",
        "“Çatı” grubunda bileşim yok; en az bir bileşim verin.")
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.contingents", "commandVersion": 1,
            "title": "Değer bileşimleri",
            "note": "ADR 0423 §1, §3: katmanın değer grupları ve kuralları. Written by scripts/fixtures/contingents_cases.py "
                    "without KentOS code; do not edit by hand.", "setup": base, "cases": cases}


# ---- cad.entities.set ----

RECORDS = {
    1: {"Tür": "konut", "Malzeme": "ahsap", "Çatı": "kirma"},
    2: {"Tür": "sanayi", "Malzeme": "beton", "Çatı": "duz"},
    3: {"Malzeme": "celik"},
    4: {"Tür": "depo", "Malzeme": "tas"},
    5: {},
}
ENTITIES = [{"kind": "polygon", "id": i, "layerId": "bina", "attrs": a, "pts": st.square(20 * i)} for i, a in RECORDS.items()]


def set_file():
    cases = []

    def step(uids, attrs, now=None, undo=False):
        now = copy.deepcopy(now or RECORDS)
        after_all = copy.deepcopy(now)
        warnings = []
        inp = {"uids": [f"$uidOf:{u}" for u in uids], "attrs": attrs, "operation": "attributes"}
        try:
            for u in uids:
                after = dict(now[u])
                for k, v in attrs.items():
                    if v is None:
                        after.pop(k, None)
                    else:
                        after[k] = v.strip()
                if after == now[u]:
                    continue
                for w in fit(FIELDS, GROUPS, now[u], after):
                    if not any(x["code"] == w["code"] for x in warnings):
                        warnings.append(w)
                after_all[u] = after
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

    case("konut her malzemeyle", [1], {"Malzeme": "tas"})
    case("sanayi betonarme ve çelikle", [2], {"Malzeme": "celik"})
    case("sanayi ahşapla reddedilir", [2], {"Malzeme": "ahsap"})
    case("türü sanayi yapınca ahşap kalırsa reddedilir", [1], {"Tür": "sanayi"})
    case("türü ve malzemesi birlikte geçerli", [1], {"Tür": "sanayi", "Malzeme": "beton"})
    case("depo taşla", [4], {"Malzeme": "tas", "Çatı": "duz"})
    case("depo malzemesiz", [4], {"Malzeme": None})
    case("depo betonla reddedilir: depo bileşimi taş ya da boş", [4], {"Malzeme": "beton"})
    case("türsüz kayıt: kısıtlayıcı grup herhangi bir türü bulamaz", [3], {"Tür": "depo"})
    case("danışma grubu: ahşap düz çatı uyarılır, yazılır", [1], {"Çatı": "duz"})
    case("danışma grubu: betonarme her çatıyla", [2], {"Çatı": "teras"})
    case("alt türe özel bileşim: sanayi çelik teras", [2], {"Malzeme": "celik", "Çatı": "teras"})
    case("alt türe özel bileşim başka türde uyarılır", [3], {"Çatı": "teras"})
    case("grubun alanlarına dokunmayan yazma sorulmaz", [1], {"Not": "Bahçeli"})
    case("boş kayda yalnız çatı: danışma grubu uyarır", [5], {"Çatı": "duz"})
    case("boş kayda yalnız tür", [5], {"Tür": "konut"})
    case("iki kayıt birden: biri bileşimi bozarsa hiçbiri yazılmaz", [1, 2], {"Malzeme": "ahsap"})
    case("iki kayıt birden geçerli", [1, 2], {"Malzeme": "beton"})
    case("iki kaydın uyarıları tek", [1, 3], {"Çatı": "teras"})
    case("grubun bütün alanları boşalır: sorulmaz", [4], {"Tür": None, "Malzeme": None})
    one, after = step([1], {"Çatı": "duz"})
    two, _ = step([1], {"Malzeme": "beton"}, now=after, undo=True)
    cases.append({"name": "uyarıdan sonra malzeme düzeltilir", "steps": [one, two]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.set", "commandVersion": 1,
            "title": "Öznitelikler ve değer bileşimleri",
            "note": "ADR 0423 §2: grubun alanlarını değiştiren yazma bir bileşime uymalı; kısıtlayıcı grup reddeder, danışma "
                    "grubu uyarır. Written by scripts/fixtures/contingents_cases.py without KentOS code.",
            "setup": setup(True, ENTITIES), "cases": cases}


# ---- cad.polygon.create ----

def create_file():
    cases = []

    def made(name, attrs=None):
        inp = {"layerId": "bina", "pts": st.square(0)}
        given = {}
        if attrs:
            inp["attrs"] = attrs
            given = {k: v.strip() for k, v in attrs.items() if v.strip()}
        out = st.new_defaults(FIELDS, SUBTYPES, given)
        try:
            st.fit(FIELDS, SUBTYPES, None, out)
            fit(FIELDS, GROUPS, None, out)
        except (Refused, st.Refused) as r:
            cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": r.error},
                                                   "expect": {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}}]})
            return
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": {"uid": "$uid", "id": 1, "revision": "$current"}, "warnings": []},
            "expect": {"ids": [1], "fields": {"1": {"attrs": out}}, "canUndo": True, "dirty": True, "revision": "changed"}}]})

    made("değersiz kayıt: ahşap varsayılanı türsüz geçer", None)
    made("konut ahşap", {"Tür": "konut"})
    made("sanayi varsayılan ahşapla reddedilir", {"Tür": "sanayi"})
    made("sanayi betonarme", {"Tür": "sanayi", "Malzeme": "beton"})
    made("sanayi çelik", {"Tür": "sanayi", "Malzeme": "celik"})
    made("depo taş", {"Tür": "depo", "Malzeme": "tas"})
    made("depo varsayılan ahşapla reddedilir", {"Tür": "depo"})
    made("depo betonla reddedilir", {"Tür": "depo", "Malzeme": "beton"})
    made("konut kırma çatı", {"Tür": "konut", "Çatı": "kirma"})
    made("konut düz çatı: yeni kayıtta danışma grubu reddetmez", {"Tür": "konut", "Çatı": "duz"})
    made("sanayi çelik teras", {"Tür": "sanayi", "Malzeme": "celik", "Çatı": "teras"})
    made("sanayi betonarme teras", {"Tür": "sanayi", "Malzeme": "beton", "Çatı": "teras"})
    made("yalnız çatı", {"Çatı": "kirma"})
    made("türsüz taş", {"Malzeme": "tas"})
    made("türsüz çelik", {"Malzeme": "celik"})
    made("boşluklu değerler", {"Tür": " sanayi ", "Malzeme": " beton "})
    made("konut taş düz", {"Tür": "konut", "Malzeme": "tas", "Çatı": "duz"})
    made("depo taş teras", {"Tür": "depo", "Malzeme": "tas", "Çatı": "teras"})
    made("sanayi ahşap verilen reddedilir", {"Tür": "sanayi", "Malzeme": "ahsap"})
    made("konut betonarme teras", {"Tür": "konut", "Malzeme": "beton", "Çatı": "teras"})
    made("notlu konut", {"Tür": "konut", "Not": "Köşe bina"})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.polygon.create", "commandVersion": 1,
            "title": "Alan çiz ve değer bileşimleri",
            "note": "ADR 0423 §2: yeni kayıt kısıtlayıcı grubun bir bileşimine uymalı. Written by "
                    "scripts/fixtures/contingents_cases.py without KentOS code.", "setup": setup(True), "cases": cases}


# ---- cad.forms.validate ----

def validate_file():
    fc, SubForm = st.form_twin()

    class GroupForm(SubForm):
        """The form twin told of the value groups too: a restrictive group narrows the choices, a touched group none of
        whose rows allows the record is the issue `form_combination`."""

        def choices(self, x, attrs, obj=None):
            base = super().choices(x, attrs, obj)
            if "lookup" in x:
                return base
            f = self.layer(x)
            grouped = None
            for g in GROUPS:
                if not g.get("restrictive") or key(f) not in g["fields"]:
                    continue
                o = offered(g, g["fields"].index(key(f)), values_of(g, FIELDS, attrs), subtype_of(attrs))
                if o is not None:
                    grouped = o if grouped is None else [c for c in grouped if c in o]
            if grouped is None:
                return base
            held = st.codes_of(f, attrs[f["name"]]) if f["name"] in attrs else []
            if base is None:
                base = [c["code"] for c in f.get("values", []) if not c.get("retired") or c["code"] in held]
            return [c for c in base if c in grouped or c in held]

        def assess(self, before, values, obj=None, new=False):
            self._new = new
            return super().assess(before, values, obj, new)

        def retired(self, changed, before, after):
            issues = super().retired(changed, before, after)
            for g in GROUPS:
                vals = values_of(g, FIELDS, after)
                if holds(g, vals, subtype_of(after)):
                    continue
                first = next((k for k in g["fields"] if k in changed or getattr(self, "_new", False)), None)
                if first is None:
                    continue
                issues.append(fc.issue("form_combination", bool(g.get("restrictive")), self.item_of(first), first))
            return issues

    body = {"revision": 1, "items": [
        {"type": "field", "id": "tur", "field": "Tür"},
        {"type": "field", "id": "malzeme", "field": "Malzeme"},
        {"type": "field", "id": "cati", "field": "Çatı"},
    ]}
    form = {"id": "bina", "name": "Bina", "layer": "bina", "draft": body,
            "releases": [{"version": 1, "digest": fc.digest(body), "body": body}]}
    S = setup(True, ENTITIES)
    S["settings"] = {**st.SETTINGS, "forms": [form]}
    cases = []

    def run(name, values=None, slot=5, defaults=False):
        f = GroupForm(body, FIELDS, SUBTYPES)
        f._new = defaults
        after, found = f.assess(dict(RECORDS[slot]), fc.utf8_sorted(dict(values or {})), {}, defaults)
        out = {"issues": fc.shown_issues(found), "valid": not fc.errors(found), "items": f.described(after, {}),
               "version": 1, "attrs": after}
        inp = {"form": "bina", "uid": f"$uidOf:{slot}"}
        if values:
            inp["values"] = values
        if defaults:
            inp["defaults"] = True
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp,
                                               "result": {"status": "completed", "output": out, "warnings": []},
                                               "expect": {"ids": list(RECORDS), "canUndo": False, "canRedo": False,
                                                          "dirty": False, "revision": "same"}}]})

    run("konut: malzeme daraltılmaz", slot=1)
    run("sanayi: malzeme betonarme ve çelik", slot=2)
    run("depo: malzeme taş", slot=4)
    run("türsüz çelik: tür seçenekleri çeliği alanlar", slot=3)
    run("boş kayıt: daraltma yok", slot=5)
    run("sanayiye ahşap: bileşim sorunu", {"Malzeme": "ahsap"}, slot=2)
    run("konutu sanayi yapmak, ahşap kalınca", {"Tür": "sanayi"}, slot=1)
    run("konutu sanayi ve betonarme yapmak", {"Tür": "sanayi", "Malzeme": "beton"}, slot=1)
    run("danışma grubu: ahşap düz çatı uyarı", {"Çatı": "duz"}, slot=1)
    run("danışma grubu: çelik teras sanayide geçerli", {"Malzeme": "celik", "Çatı": "teras"}, slot=2)
    run("danışma grubu: çelik teras türsüz kayıtta uyarı", {"Çatı": "teras"}, slot=3)
    run("depoya betonarme: sorun", {"Malzeme": "beton"}, slot=4)
    run("depoyu boşaltmak", {"Malzeme": ""}, slot=4)
    run("yeni kayıt, sanayi: varsayılan ahşap sorun", {"Tür": "sanayi"}, defaults=True)
    run("yeni kayıt, sanayi çelik", {"Tür": "sanayi", "Malzeme": "celik"}, defaults=True)
    run("yeni kayıt, depo taş", {"Tür": "depo", "Malzeme": "tas"}, defaults=True)
    run("yeni kayıt, varsayılanlarla", defaults=True)
    run("dokunulmayan grup sorulmaz: türsüz kayda çatı yok", {"Tür": "konut"}, slot=3)
    run("malzemesi çelik türsüz kayda depo: sorun", {"Tür": "depo"}, slot=3)
    run("sanayi: çatı seçenekleri daraltılmaz (danışma)", slot=2)
    run("konutun ahşabı düzden kırmaya", {"Çatı": "kirma"}, slot=1)
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.validate", "commandVersion": 1,
            "title": "Formu denetle ve değer bileşimleri",
            "note": "ADR 0423 §2: kısıtlayıcı grup seçenekleri daraltır; dokunulan grubun bileşimi yoksa form_combination. "
                    "Written by scripts/fixtures/contingents_cases.py with the form twin of forms_cases.py, without KentOS "
                    "code; the issues' sentences are the core's ($any).", "setup": S, "cases": cases}


def outputs():
    return {OUT / "cad.layers.contingents.json": contingents_file(), OUT / "cad.entities.set.contingents.json": set_file(),
            OUT / "cad.polygon.create.contingents.json": create_file(),
            OUT / "cad.forms.validate.contingents.json": validate_file()}


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
