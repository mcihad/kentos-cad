"""Writes fixtures/commands/v1/cad.domains.edit.json: the shared cases of `cad.domains.edit` v1 (Değer listeleri,
docs/adr/0430), their expectations built here without KentOS code from the ADR's rule: a project's value list is a coded
list or a range; a linked field's values (or bounds) are its list's copy, written anew in the step that changes the list;
a code records hold may not leave a list unless `recode` moves them; a list fields link is not removed unless `detach`.

    python3 scripts/fixtures/domains_cases.py           # writes the file
    python3 scripts/fixtures/domains_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from decimal import Decimal, InvalidOperation
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the revision words, written without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.domains.edit.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
KIND_WORDS = {"text": "metin", "integer": "tam sayı", "decimal": "ondalık sayı", "date": "tarih", "boolean": "evet/hayır",
              "datetime": "tarih ve saat", "time": "saat"}


def ch(code, label, retired=False):
    return {"code": code, "label": label, **({"retired": True} if retired else {})}


MALZEME = [ch("ah", "Ahşap"), ch("bt", "Betonarme"), ch("ym", "Yığma")]
DOMAINS = [
    {"id": "malzeme", "name": "Yapı malzemesi", "kind": "text", "values": copy.deepcopy(MALZEME)},
    {"id": "kat", "name": "Kat sayısı", "kind": "integer", "min": "1", "max": "40"},
    {"id": "genislik", "name": "Yol genişliği", "kind": "decimal", "values": [ch("2.5", "Dar"), ch("3.5", "Geniş")]},
]


def layer(i, name, fields=None, kind="layer", children=None, locked=False):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields is not None:
        out["fields"] = fields
    return out


def linked(name, domain):
    d = next(x for x in DOMAINS if x["id"] == domain)
    return {"name": name, "kind": d["kind"], **{k: copy.deepcopy(d[k]) for k in ("min", "max", "values") if k in d},
            "domain": domain}


LAYERS = [
    layer("bina", "Bina", [
        {"name": "Malzeme", "kind": "text", "values": copy.deepcopy(MALZEME)},
        {"id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40"},
        {"name": "Etiket", "kind": "text", "multiple": True, "values": [ch("a", "A"), ch("b", "B")]},
        {"name": "Not", "kind": "text"},
    ]),
    layer("depo", "Depo", [linked("Malzeme", "malzeme")]),
    layer("grup", "Eski yapılar", kind="group", children=[layer("eski", "Eski", [linked("Malzeme", "malzeme")],
                                                                    locked=True)]),
    layer("yol", "Yol", [{**linked("Genişlik", "genislik"), "scale": 1}]),
]
SQUARE = [{"x": 0, "y": 0}, {"x": 10, "y": 0}, {"x": 10, "y": 10}, {"x": 0, "y": 10}]
ENTITIES = [
    (1, "bina", {"Malzeme": "ah", "Kat": "3", "Etiket": "a; b"}),
    (2, "bina", {"Malzeme": "bt", "Kat": "45"}),
    (3, "depo", {"Malzeme": "ym"}),
    (4, "eski", {"Malzeme": "ah"}),
    (5, "yol", {"Genişlik": "2.5"}),
]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Değer listeleri",
            "settings": {**SETTINGS, "domains": copy.deepcopy(DOMAINS)},
            "origin": {"x": 487000, "y": 4420000}, "layers": copy.deepcopy(LAYERS), "activeLayer": "bina",
            "entities": [{"kind": "polygon", "id": i, "layerId": lay, "pts": copy.deepcopy(SQUARE), "attrs": attrs}
                         for i, lay, attrs in ENTITIES],
            "styles": {"items": [], "categories": []}}


# ---- The rule, written from the ADR ----

class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def key(f):
    return f.get("id", f["name"])


def leaves(nodes):
    for n in nodes:
        if n["type"] == "layer":
            yield n
        yield from leaves(n["children"])


def node(nodes, i):
    for n in nodes:
        if n["id"] == i:
            return n
        found = node(n["children"], i)
        if found:
            return found
    return None


def canonical(kind, scale, v):
    """Whether `v` is a number's canonical text for a field of `kind` (and `scale` digits)."""
    try:
        d = Decimal(v)
    except InvalidOperation:
        return False
    if kind == "integer":
        return v == str(int(d)) if d == int(d) else False
    if v.startswith("+") or (v.startswith("0") and len(v) > 1 and v[1] != ".") or v.endswith("."):
        return False
    digits = len(v.split(".")[1]) if "." in v else 0
    return scale is None or digits <= scale


def field_words(f):
    """The rules of a field's list and range that the cases reach, in the contract's order and words."""
    name, kind = f["name"], f["kind"]
    if "min" in f or "max" in f:
        if kind not in ("integer", "decimal"):
            return f"“{name}” alanında aralık yalnız sayı, tarih ve saat alanlarında olur."
        if "min" in f and "max" in f and Decimal(f["min"]) > Decimal(f["max"]):
            return f"“{name}” alanının en azı en çoğundan büyük olamaz."
    if "values" in f:
        vs = f["values"]
        if not vs:
            return f"“{name}” alanının değer listesi boş; listeyi kaldırın ya da değer ekleyin."
        for i, v in enumerate(vs):
            if kind != "text" and not canonical(kind, f.get("scale"), v["code"]):
                return f"“{name}” alanının değer listesindeki “{v['code']}” kodu alanın türüne uymuyor."
            if any(w["code"] == v["code"] for w in vs[:i]):
                return f"“{name}” alanının değer listesinde “{v['code']}” kodu iki kez var."
    return None


def domain_words(d):
    ranged = "min" in d or "max" in d
    if "values" in d and ranged:
        return f"“{d['name']}” listesi ya kod listesidir ya aralıktır; ikisi birden olamaz."
    if "values" not in d and not ranged:
        return f"“{d['name']}” listesinde ne kod var ne aralık; kod ekleyin ya da en azını veya en çoğunu verin."
    if "values" in d and d["values"] and all(c.get("retired") for c in d["values"]):
        return f"“{d['name']}” listesinin bütün kodları kullanımdan kalkmış; en az biri kullanımda olmalı."
    words = field_words({"name": d["name"], "kind": d["kind"], **{k: d[k] for k in ("min", "max", "values") if k in d}})
    if words:
        words = words.replace("alanının", "listesinin", 1).replace("alanında", "listesinde", 1)
    return words


def domains_words(ds):
    for i, d in enumerate(ds):
        w = domain_words(d)
        if w:
            return w
        if any(o["id"] == d["id"] for o in ds[:i]):
            return f"“{d['id']}” kimlikli iki değer listesi var."
        if any(o["name"].lower() == d["name"].lower() for o in ds[:i]):
            return f"“{d['name']}” adlı iki değer listesi var."
    return None


def applied(d, f):
    out = {k: v for k, v in f.items() if k not in ("values", "min", "max", "domain")}
    out.update({k: copy.deepcopy(d[k]) for k in ("values", "min", "max") if k in d})
    out["domain"] = d["id"]
    return out


def link_words(d, f):
    if f["kind"] != d["kind"]:
        label = f.get("alias", f["name"])
        return f"“{d['name']}” listesi {KIND_WORDS[d['kind']]} alanına uyar; “{label}” {KIND_WORDS[f['kind']]} alanı."
    return field_words(applied(d, f))


def follow(fields, domains):
    out = []
    for f in fields:
        d = next((x for x in domains if x["id"] == f.get("domain")), None)
        out.append(applied(d, f) if d and not link_words(d, f) else f)
    return out


def domain_id_of(name, taken):
    fold = str.maketrans("çÇğĞıIİöÖşŞüÜ", "ccggiiioossuu")
    stem = ""
    for c in name.translate(fold).lower():
        if c.isascii() and c.isalnum():
            stem += c
        elif stem and not stem.endswith("-"):
            stem += "-"
    stem = stem.rstrip("-")[:56] or "liste"
    if stem not in taken:
        return stem
    n = 2
    while f"{stem}-{n}" in taken:
        n += 1
    return f"{stem}-{n}"


def uses(layers, i):
    return [(n["id"], key(f)) for n in leaves(layers) for f in n.get("fields", []) if f.get("domain") == i]


def field_at(state, lay, k):
    return next(f for f in node(state["layers"], lay)["fields"] if key(f) == k)


def codes_in(f, value):
    if f.get("multiple") and f["kind"] == "text":
        return [c.strip() for c in value.split(";") if c.strip()]
    return [value]


def values_of(state, lay, name):
    return [(i, a[name]) for i, l2, a in state["entities"] if l2 == lay and a.get(name)]


def misfits(state, used, values, recode):
    n, lost = 0, set()
    for lay, k in used:
        f = field_at(state, lay, k)
        for _, v in values_of(state, lay, f["name"]):
            gone = [recode.get(c, c) for c in codes_in(f, v)]
            gone = [c for c in gone if not any(x["code"] == c for x in values)]
            if gone:
                n += 1
                lost |= set(gone)
    return n, lost


def outside(state, used, d):
    n = 0
    for lay, k in used:
        f = field_at(state, lay, k)
        for _, v in values_of(state, lay, f["name"]):
            n += ("min" in d and Decimal(v) < Decimal(d["min"])) or ("max" in d and Decimal(v) > Decimal(d["max"]))
    return n


def warning(n):
    return [{"code": "domain_values_outside", "message": f"{n} kaydın değeri listenin aralığı dışında kaldı.",
             "path": "edit"}] if n else []


def in_use(n, lost, path):
    return Refused("domain_code_in_use", f"{n} kayıt listeden çıkan kodları tutuyor ({', '.join(sorted(lost))}); recode "
                   "ile başka bir koda taşıyın ya da kodu kullanımdan kaldırın.", path)


def checked_field(state, lay, k):
    n = node(state["layers"], lay)
    if n is None:
        raise Refused("layer_not_found", f"“{lay}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "edit/layer")
    if n["type"] != "layer":
        raise Refused("not_a_layer", f"“{n['name']}” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını "
                      "verin.", "edit/layer")
    f = next((x for x in n.get("fields", []) if key(x) == k), None)
    if f is None:
        raise Refused("field_not_found", f"“{n['name']}” katmanında “{k}” alanı yok.", "edit/field")
    return f


def unknown(i, path):
    return Refused("unknown_domain", f"“{i}” kimlikli bir değer listesi yok. Değer listeleri penceresinden var olan bir "
                   "listeyi seçin.", path)


def plan(state, edit):
    """(next domains, links [(layer, field)], links first, recodes {id: attrs}, domain, warnings), or Refused."""
    now = state["domains"]
    find = lambda i: next((d for d in now if d["id"] == i), None)  # noqa: E731

    def taken(i):
        if i is not None and find(i):
            raise Refused("domain_exists", f"“{i}” kimlikli bir değer listesi zaten var; başka bir kimlik verin.",
                          "edit/id")

    def fits(ds):
        w = domains_words(ds)
        if w:
            raise Refused("invalid_domain", w, "edit")

    def fresh(i, name):
        return i if i is not None else domain_id_of(name, [d["id"] for d in now])

    kind = edit["kind"]
    if kind == "create":
        taken(edit.get("id"))
        d = {"id": fresh(edit.get("id"), edit["name"]), "name": edit["name"],
             **({"description": edit["description"]} if "description" in edit else {}), "kind": edit["domainKind"],
             **{k: copy.deepcopy(edit[k]) for k in ("values", "min", "max") if k in edit}}
        nxt = now + [d]
        fits(nxt)
        return nxt, [], False, {}, d["id"], []
    if kind == "fromField":
        f = checked_field(state, edit["layer"], edit["field"])
        taken(edit.get("id"))
        if not any(k in f for k in ("values", "min", "max")):
            raise Refused("invalid_domain", f"“{f.get('alias', f['name'])}” alanının ne değer listesi ne aralığı var; "
                          "listesi olan bir alan seçin.", "edit/field")
        d = {"id": fresh(edit.get("id"), edit["name"]), "name": edit["name"], "kind": f["kind"],
             **({"values": copy.deepcopy(f["values"])} if "values" in f else
                {k: f[k] for k in ("min", "max") if k in f})}
        nxt = now + [d]
        fits(nxt)
        return nxt, [(edit["layer"], {**f, "domain": d["id"]})], False, {}, d["id"], []
    if kind == "update":
        old = find(edit["id"])
        if old is None:
            raise unknown(edit["id"], "edit/id")
        d = {"id": old["id"], "name": edit["name"], **({"description": edit["description"]} if "description" in edit else {}),
             "kind": old["kind"], **{k: copy.deepcopy(edit[k]) for k in ("values", "min", "max") if k in edit}}
        nxt = [d if o["id"] == old["id"] else o for o in now]
        fits(nxt)
        used = uses(state["layers"], old["id"])
        for lay, k in used:
            w = link_words(d, field_at(state, lay, k))
            if w:
                raise Refused("domain_mismatch", w, "edit")
        recode = edit.get("recode", {})
        for frm, to in sorted(recode.items()):
            if not any(c["code"] == frm for c in old.get("values", [])):
                raise Refused("invalid_recode", f"“{frm}” kodu “{old['name']}” listesinde yok.", f"edit/recode/{frm}")
            if not any(c["code"] == to and not c.get("retired") for c in d.get("values", [])):
                raise Refused("invalid_recode", f"“{to}” kodu yeni listede yok ya da kullanımdan kalkmış.",
                              f"edit/recode/{frm}")
        warnings = []
        if "values" in d:
            n, lost = misfits(state, used, d["values"], recode)
            if n:
                raise in_use(n, lost, "edit/values")
        else:
            warnings = warning(outside(state, used, d))
        recodes = {}
        for lay, k in used:
            f = field_at(state, lay, k)
            for i, v in values_of(state, lay, f["name"]):
                codes = codes_in(f, v)
                if not any(c in recode for c in codes):
                    continue
                if node(state["layers"], lay)["locked"]:
                    raise Refused("layer_locked", f"“{node(state['layers'], lay)['name']}” katmanı kilitli; kayıtlarının "
                                  "kodu değiştirilemez. Kilidi Katmanlar panelinden açın.", "edit/recode")
                moved = []
                for c in codes:
                    c = recode.get(c, c)
                    if c not in moved:
                        moved.append(c)
                recodes[i] = (f["name"], "; ".join(moved))
        return nxt, [], False, recodes, old["id"], warnings
    if kind == "remove":
        old = find(edit["id"])
        if old is None:
            raise unknown(edit["id"], "edit/id")
        used = uses(state["layers"], old["id"])
        if used and not edit.get("detach"):
            said = ", ".join(f"{node(state['layers'], lay)['name']} › {field_at(state, lay, k).get('alias', field_at(state, lay, k)['name'])}"
                             for lay, k in used)
            raise Refused("domain_in_use", f"“{old['name']}” listesine {len(used)} alan bağlı ({said}); bağları çözün ya da "
                          "detach ile silin.", "edit/id")
        links = [(lay, {k2: v for k2, v in field_at(state, lay, k).items() if k2 != "domain"}) for lay, k in used]
        return [d for d in now if d["id"] != old["id"]], links, True, {}, old["id"], []
    if kind == "link":
        d = None
        if edit.get("domain") is not None:
            d = find(edit["domain"])
            if d is None:
                raise unknown(edit["domain"], "edit/domain")
        f = checked_field(state, edit["layer"], edit["field"])
        warnings = []
        if d:
            w = link_words(d, f)
            if w:
                raise Refused("domain_mismatch", w, "edit/field")
            used = [(edit["layer"], edit["field"])]
            if "values" in d:
                n, lost = misfits(state, used, d["values"], {})
                if n:
                    raise in_use(n, lost, "edit/domain")
            else:
                warnings = warning(outside(state, used, d))
        links = []
        if f.get("domain") != edit.get("domain"):
            g = {k2: v for k2, v in f.items() if k2 != "domain"}
            if edit.get("domain") is not None:
                g["domain"] = edit["domain"]
            links = [(edit["layer"], g)]
        return now, links, False, {}, edit.get("domain"), warnings
    raise AssertionError(kind)


def applied_state(state, p):
    """The state after the plan `p`, and the fields whose definition changed."""
    domains, links, _, recodes, _, _ = p
    after = copy.deepcopy(state)
    after["domains"] = copy.deepcopy(domains)
    changed = []
    for n in leaves(after["layers"]):
        if "fields" not in n:
            continue
        given = [next((g for lay, g in links if lay == n["id"] and key(g) == key(f)), f) for f in n["fields"]]
        new = follow(given, domains)
        changed += [{"layer": n["id"], "field": key(a)} for a, b in zip(n["fields"], new) if a != b]
        n["fields"] = new
    after["entities"] = [(i, lay, {**a, recodes[i][0]: recodes[i][1]} if i in recodes else a)
                         for i, lay, a in after["entities"]]
    return after, changed


def answer(state, p):
    after, changed = applied_state(state, p)
    domains, _, _, recodes, domain, _ = p
    out = {**({"domain": domain} if domain is not None else {}), "fields": changed, "recoded": len(recodes),
           "changed": domains != state["domains"] or bool(changed) or bool(recodes), "revision": "$current"}
    return after, out


UNCHANGED = {"canUndo": False, "dirty": False, "revision": "same"}


def first_state():
    return {"domains": copy.deepcopy(DOMAINS), "layers": copy.deepcopy(LAYERS),
            "entities": [(i, lay, dict(a)) for i, lay, a in ENTITIES]}


def expect_of(state, written, recoded=()):
    out = {"canUndo": written, "dirty": written, "revision": "changed" if written else "same",
           "settings": {"domains": state["domains"] or None}, "layers": state["layers"]}
    if recoded:
        out["fields"] = {str(i): {"attrs": a} for i, _, a in state["entities"] if i in recoded}
    return out


def step(state, edit, op="execute", **more):
    inp = {"edit": edit, **more}
    try:
        p = plan(state, edit)
    except Refused as r:
        return {"op": op, "input": inp, "result": fc.failed(r.code, r.message, r.path), "expect": UNCHANGED}, state
    after, out = answer(state, p)
    warnings = p[5]
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": UNCHANGED}, state
    if op == "plan":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
                "expect": UNCHANGED}, state
    written = out["changed"]
    return {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
            "expect": expect_of(after if written else state, written, tuple(p[3]))}, after if written else state


def one(name, edit, op="execute", **more):
    s, _ = step(first_state(), edit, op, **more)
    return {"name": name, "steps": [s]}


def cases():
    out = [
        one("kod listesi: kimlik adından, Türkçe harfler katlanır", {"kind": "create", "name": "Çatı türü",
            "domainKind": "text", "values": [ch("dz", "Düz"), ch("bh", "Beşik")]}),
        one("aralık: verilen kimlikle, açıklamasıyla", {"kind": "create", "id": "yas", "name": "Bina yaşı",
            "description": "Yapım yılından bu yana.", "domainKind": "integer", "min": "0", "max": "300"}),
        one("adı alınmış kimlik çoğaltılır", {"kind": "create", "name": "Yapı malzemesi 2", "domainKind": "text",
                                               "values": [ch("x", "X")]}),
        one("kimlik zaten var", {"kind": "create", "id": "kat", "name": "Başka", "domainKind": "integer", "min": "0"}),
        one("hem kod hem aralık olamaz", {"kind": "create", "name": "Karışık", "domainKind": "integer",
                                          "values": [ch("1", "Bir")], "min": "0"}),
        one("ne kod ne aralık", {"kind": "create", "name": "Boş", "domainKind": "text"}),
        one("aralığın en azı en çoğundan büyük", {"kind": "create", "name": "Ters", "domainKind": "integer", "min": "9",
                                                  "max": "2"}),
        one("aynı kod iki kez", {"kind": "create", "name": "Çift", "domainKind": "text",
                                 "values": [ch("a", "A"), ch("a", "B")]}),
        one("metinde aralık olmaz", {"kind": "create", "name": "Metin aralığı", "domainKind": "text", "min": "1"}),
        one("adı başka listeninkiyle aynı (büyük küçük harf yok sayılır)", {"kind": "create", "name": "kat sayısı",
                                                                            "domainKind": "integer", "min": "0"}),
        one("alanın listesinden: liste yapılır, alan bağlanır", {"kind": "fromField", "layer": "bina", "field": "Malzeme",
                                                                 "name": "Bina malzemesi"}),
        one("alanın aralığından", {"kind": "fromField", "layer": "bina", "field": "kat-sayisi", "name": "Bina katı",
                                   "id": "bina-kat"}),
        one("listesi olmayan alan", {"kind": "fromField", "layer": "bina", "field": "Not", "name": "Not listesi"}),
        one("katman yok", {"kind": "fromField", "layer": "yok", "field": "Malzeme", "name": "X"}),
        one("grup katman değil", {"kind": "fromField", "layer": "grup", "field": "Malzeme", "name": "X"}),
        one("alan yok", {"kind": "fromField", "layer": "bina", "field": "Renk", "name": "X"}),
        one("güncelle: etiket değişir, kod eklenir; bağlı alanlar izler (kilitli katmanınki de)",
            {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
             "values": [ch("ah", "Ahşap yapı"), ch("bt", "Betonarme"), ch("ym", "Yığma"), ch("ce", "Çelik")]}),
        one("kayıtların tuttuğu kod listeden çıkamaz", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
                                                         "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme")]}),
        one("kod eşlemesiyle çıkar: kayıt aynı adımda taşınır", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme")], "recode": {"ym": "bt"}}),
        one("kullanımdan kaldırılan kod kalır", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme"), ch("ym", "Yığma", True)]}),
        one("eşlemenin kodu eski listede yok", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": copy.deepcopy(MALZEME), "recode": {"zz": "bt"}}),
        one("eşlemenin hedefi kullanımdan kalkmış", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme", True), ch("ym", "Yığma")], "recode": {"ym": "bt"}}),
        one("kilitli katmanın kaydı taşınamaz", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": copy.deepcopy(MALZEME), "recode": {"ah": "bt"}}),
        one("ondalık kod alanın basamağına sığmaz", {"kind": "update", "id": "genislik", "name": "Yol genişliği",
            "values": [ch("2.5", "Dar"), ch("3.5", "Geniş"), ch("3.75", "Çok geniş")]}),
        one("olmayan liste güncellenemez", {"kind": "update", "id": "yok", "name": "X", "values": [ch("a", "A")]}),
        one("aralığa bağlama: dışarıda kalan kayıt uyarıyla", {"kind": "link", "layer": "bina", "field": "kat-sayisi",
                                                               "domain": "kat"}),
        one("başka türdeki alan bağlanamaz", {"kind": "link", "layer": "bina", "field": "Not", "domain": "kat"}),
        one("çok değerli alanın kayıtları listede olmayan kodları tutuyor", {"kind": "link", "layer": "bina",
                                                                              "field": "Etiket", "domain": "malzeme"}),
        one("bağlama: kendi listesi listeninkiyle değişir", {"kind": "link", "layer": "bina", "field": "Malzeme",
                                                            "domain": "malzeme"}),
        one("olmayan listeye bağlanamaz", {"kind": "link", "layer": "bina", "field": "Malzeme", "domain": "yok"}),
        one("bağı çöz: kopya alanın kendi listesi olur", {"kind": "link", "layer": "depo", "field": "Malzeme"}),
        one("bağlı olmayan alanın bağı çözülmez: değişiklik yok", {"kind": "link", "layer": "bina", "field": "Malzeme"}),
        one("bağlı alanları olan liste silinmez", {"kind": "remove", "id": "malzeme"}),
        one("detach: alanlar kopyalarını tutar, liste gider", {"kind": "remove", "id": "malzeme", "detach": True}),
        one("bağlı alanı olmayan liste silinir", {"kind": "remove", "id": "kat"}),
        one("olmayan liste silinmez", {"kind": "remove", "id": "yok"}),
        one("plan aynı cevabı verir, hiçbir şey yazmaz", {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
            "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme")], "recode": {"ym": "bt"}}, op="plan"),
        one("doğrulama hiçbir şey yazmaz", {"kind": "remove", "id": "kat"}, op="validate"),
        one("geçersiz sürüm", {"kind": "remove", "id": "kat"}, expectedRevision="07"),
    ]
    # The revision words: invalid and stale.
    out[-1]["steps"][0].update(result=fc.failed("invalid_revision", fc.REVISION, "expectedRevision"), expect=UNCHANGED)
    conflict = one("sürüm çakışması", {"kind": "remove", "id": "kat"}, expectedRevision="999")
    conflict["steps"][0].update(result=fc.failed("revision_conflict", fc.CONFLICT, "expectedRevision", "conflict"),
                                expect=UNCHANGED)
    out.append(conflict)
    # Undo gives the lists, the copies and the records back.
    s0 = first_state()
    a, s1 = step(s0, {"kind": "update", "id": "malzeme", "name": "Yapı malzemesi",
                      "values": [ch("ah", "Ahşap"), ch("bt", "Betonarme")], "recode": {"ym": "bt"}})
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "settings": {"domains": s0["domains"]}, "layers": s0["layers"],
                       "fields": {"3": {"attrs": {"Malzeme": "ym"}}}}}
    out.append({"name": "geri alma listeyi, kopyaları ve kayıtları geri verir", "steps": [a, undo]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.domains.edit", "commandVersion": 1,
            "title": "Değer listeleri",
            "note": "ADR 0430: projenin değer listeleri ve alanların bağı. Written by scripts/fixtures/domains_cases.py "
                    "without KentOS code; do not edit by hand.", "setup": setup(), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = len(json.loads(text)["cases"])
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({n} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({n} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
