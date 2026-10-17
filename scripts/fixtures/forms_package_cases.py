"""Writes the shared cases of a form package (docs/adr/0435) without KentOS code:

- fixtures/commands/v1/cad.forms.export.json: `cad.forms.export` v1, the package of a form: its latest release (or its
  draft), the fields it binds or reads (the items' keys, the names in its expressions and templates, its workflow's
  fields) in the layer's order, and the value lists those fields link;
- fixtures/commands/v1/cad.forms.import.json: `cad.forms.import` v1, a package made a new form of a layer: its fields
  bound by `map`, else the same key, else the same name (the same kind), the body's keys and names rewritten all at once,
  the missing fields added with their lists when `addFields`, else refused.

The places that name a field are field_usage_cases.py's (written without KentOS code too).

    python3 scripts/fixtures/forms_package_cases.py           # writes the files
    python3 scripts/fixtures/forms_package_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import field_usage_cases as fu  # noqa: E402  (the places that name a field, and renaming in expressions)
import forms_cases as fc  # noqa: E402  (the digest and the revision words)

OUT = ROOT / "fixtures/commands/v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
FORMAT, VERSION = "kentos.form-package", 1
KINDS = {"text": "metin", "integer": "tam sayı", "decimal": "ondalık sayı", "date": "tarih", "boolean": "evet/hayır"}


def ch(code, label):
    return {"code": code, "label": label}


DURUM = [ch("iyi", "İyi"), ch("inceleme", "Ek inceleme gerekli")]
MALZEME = [ch("ah", "Ahşap"), ch("bt", "Betonarme")]
DOMAINS = [{"id": "malzeme", "name": "Yapı malzemesi", "kind": "text", "values": copy.deepcopy(MALZEME)}]
BINA = [
    {"name": "Durum", "kind": "text", "values": copy.deepcopy(DURUM)},
    {"id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1"},
    {"name": "Malzeme", "kind": "text", "values": copy.deepcopy(MALZEME), "domain": "malzeme"},
    {"name": "Not", "kind": "text"},
    {"name": "Gerekçe", "kind": "text"},
    {"name": "Renk", "kind": "text"},
]
YAPI = [
    {"name": "Durumu", "kind": "text", "values": copy.deepcopy(DURUM)},
    {"name": "KatSayisi", "kind": "integer"},
    {"name": "Malzemesi", "kind": "text"},
    {"name": "Açıklama", "kind": "text"},
]
KAT = [{"name": "Durum", "kind": "integer"}]


def layer(i, name, fields=None, kind="layer", children=None):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields is not None:
        out["fields"] = copy.deepcopy(fields)
    return out


LAYERS = [layer("bina", "Bina", BINA), layer("yapi", "Yapı", YAPI), layer("bos", "Boş katman"), layer("kat", "Kat katmanı", KAT),
          layer("grup", "Grup", kind="group", children=[layer("ic", "İç")])]

DENETIM = {"revision": 3, "items": [
    {"type": "field", "id": "durum", "field": "Durum", "required": True},
    {"type": "field", "id": "kat", "field": "kat-sayisi", "visibleWhen": "Durum = 'inceleme'"},
    {"type": "field", "id": "malzeme", "field": "Malzeme"},
    {"type": "note", "id": "bilgi", "text": "Kat: {Kat}"},
    {"type": "field", "id": "not", "field": "Not", "requiredWhen": "Kat > 5"},
]}
DENETIM_V1 = {"revision": 1, "items": [{"type": "field", "id": "durum", "field": "Durum"}]}
OKUR = {"revision": 1, "items": [{"type": "field", "id": "durum", "field": "Durum", "visibleWhen": "Kat > 2"}]}
AKIS = {"revision": 2, "items": [{"type": "field", "id": "durum", "field": "Durum"},
                                 {"type": "field", "id": "gerekce", "field": "Gerekçe"}],
        "workflow": {"field": "Durum", "initial": "inceleme", "steps": [
            {"label": "Onayla", "from": ["inceleme"], "to": "iyi", "valid": True},
            {"label": "Geri al", "from": ["iyi"], "to": "inceleme", "reason": "Gerekçe"}]}}
NOTLU = {"revision": 1, "items": [{"type": "note", "id": "yalniz", "text": "Bu formda alan yok."}]}


def release(body, v):
    return {"version": v, "digest": fc.digest(body), "body": body}


FORMS = [
    {"id": "denetim", "name": "Bina denetimi", "layer": "bina", "draft": copy.deepcopy(DENETIM),
     "releases": [release(DENETIM_V1, 1), release(DENETIM, 2)]},
    {"id": "taslak", "name": "Taslak form", "layer": "bina", "draft": copy.deepcopy(OKUR)},
    {"id": "okur", "name": "Okuyan form", "layer": "bina", "draft": copy.deepcopy(OKUR), "releases": [release(OKUR, 1)]},
    {"id": "akis", "name": "Onay akışı", "layer": "bina", "draft": copy.deepcopy(AKIS), "releases": [release(AKIS, 1)]},
    {"id": "notlu", "name": "Notlu form", "layer": "bina", "draft": copy.deepcopy(NOTLU), "releases": [release(NOTLU, 1)]},
]


def setup(forms=None, domains=None):
    return {"format": "kentos.document", "version": 1, "name": "Form paketi",
            "settings": {**SETTINGS, "forms": copy.deepcopy(FORMS if forms is None else forms),
                         "domains": copy.deepcopy(DOMAINS if domains is None else domains)},
            "origin": {"x": 487000, "y": 4420000}, "layers": copy.deepcopy(LAYERS), "activeLayer": "bina",
            "entities": [], "styles": {"items": [], "categories": []}}


UNCHANGED = {"canUndo": False, "dirty": False, "revision": "same"}


# ---- The rule, written from the ADR ----

def key(f):
    return f.get("id", f["name"])


def own_sites(body, fields):
    """The places of `body` naming its own layer's fields: (how, path, value)."""
    out = []
    fu.items(out, [{"id": "paket", "type": "layer", "fields": fields, "children": []}], "paket", body["items"], "items",
             "Paket", False)
    return [(how, path, value) for lid, how, path, _, value, _ in out if lid == "paket"]


def mentions(how, value, name):
    if how == "expression":
        return fu.names(value, name)
    return any(fu.names(value[s:e], name) for s, e in fu.template_exprs(value) or [])


def read_by(body, fields):
    keys, names = [], []
    for how, _, value in own_sites(body, fields):
        if how == "key":
            keys.append(value)
        elif how == "name":
            names.append(value)
        else:
            names += [f["name"] for f in fields if mentions(how, value, f["name"])]
    w = body.get("workflow")
    if w:
        keys += [w["field"]] + ([w["log"]] if "log" in w else []) + [s["reason"] for s in w["steps"] if "reason" in s]
    return [key(f) for f in fields if key(f) in keys or f["name"] in names]


def set_at(doc, path, value):
    cur = doc
    parts = path.split("/")
    for p in parts[:-1]:
        cur = cur[int(p)] if isinstance(cur, list) else cur[p]
    cur[parts[-1]] = value


def rebind(body, pairs, fields):
    """`body` with the fields rebound at once: pairs are (from key, from name, to key, to name)."""
    out = copy.deepcopy(body)
    stand_in = lambda i: f"kentos_paket_{i}"  # noqa: E731
    for how, path, value in own_sites(out, fields):
        if how == "key":
            p = next((p for p in pairs if p[0] == value), None)
            if p:
                set_at(out, path, p[2])
        elif how in ("expression", "template"):
            v = value
            for i, p in enumerate(pairs):
                r = fu.rename(v, p[1], stand_in(i)) if how == "expression" else fu.template_rename(v, p[1], stand_in(i))
                v = r if r is not None else v
            for i, p in enumerate(pairs):
                r = fu.rename(v, stand_in(i), p[3]) if how == "expression" else fu.template_rename(v, stand_in(i), p[3])
                v = r if r is not None else v
            set_at(out, path, v)
    w = out.get("workflow")
    rekey = lambda k: next((p[2] for p in pairs if p[0] == k), k)  # noqa: E731
    if w:
        w["field"] = rekey(w["field"])
        if "log" in w:
            w["log"] = rekey(w["log"])
        for s in w["steps"]:
            if "reason" in s:
                s["reason"] = rekey(s["reason"])
    out["revision"] = 1
    return out


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def package_of(doc, form_id, draft=False):
    form = next((f for f in doc["settings"]["forms"] if f["id"] == form_id), None)
    if form is None:
        raise Refused("unknown_form", f"“{form_id}” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.",
                      "form")
    if draft:
        body = form["draft"]
    elif form.get("releases"):
        body = form["releases"][-1]["body"]
    else:
        raise Refused("form_unpublished", f"“{form['name']}” formu hiç yayınlanmadı; taslağı draft ile dışa aktarın ya da önce "
                      "yayınlayın.", "draft")
    fields = fu.find(doc["layers"], form["layer"]).get("fields", [])
    keys = read_by(body, fields)
    chosen = [copy.deepcopy(f) for f in fields if key(f) in keys]
    domains = []
    for f in chosen:
        d = next((d for d in doc["settings"].get("domains", []) if d["id"] == f.get("domain")), None)
        if d and d not in domains:
            domains.append(copy.deepcopy(d))
    out = {"format": FORMAT, "version": VERSION, "name": form["name"], "body": copy.deepcopy(body), "fields": chosen}
    if domains:
        out["domains"] = domains
    return out


def bound_keys(items):
    for i in items:
        if i["type"] == "field":
            yield i["field"]
        yield from bound_keys(i.get("items", []))


def item_id_of(stem, taken):
    """kentos_forms' item_id_of: folded the Turkish way, letters and digits kept, `-` between, at most 60."""
    s = ""
    for c in fc.fold(stem):
        if c.isalnum():
            s += c
        elif not s.endswith("-"):
            s += "-"
    s = s.strip("-")[:60] or "alan"
    if s not in taken:
        return s
    n = 2
    while f"{s}-{n}" in taken:
        n += 1
    return f"{s}-{n}"


def imported(doc, inp):
    """(form, bound, added keys, added domain ids, layer fields afterwards or None, domains afterwards or None,
    warnings), or Refused."""
    p = inp["package"]
    if p["format"] != FORMAT or p["version"] != VERSION:
        raise Refused("invalid_package", f"Paketin biçimi “{p['format']}” sürüm {p['version']}; KentOS form paketi "
                      f"({FORMAT}, sürüm {VERSION}) değil.", "package")
    missing_key = next((k for k in bound_keys(p["body"]["items"]) if not any(key(f) == k for f in p["fields"])), None)
    if missing_key:
        raise Refused("invalid_package", f"Paketin formu “{missing_key}” alanını anıyor; alan paketin alanlarında yok.",
                      "package.body")
    node = fu.find(doc["layers"], inp["layer"])
    if node is None:
        raise Refused("layer_not_found", f"“{inp['layer']}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "layer")
    if node["type"] != "layer":
        raise Refused("not_a_layer", f"“{node['name']}” nesne tutan bir katman değil; form yalnız katmanın olur. Bir katman "
                      "seçin.", "layer")
    name = inp.get("name", p["name"])
    if not name.strip() or len(name.strip()) > 64:
        raise Refused("invalid_form_name", "Form adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir "
                      "ad yazın.", "name")
    taken = [f["id"] for f in doc["settings"]["forms"]]
    if "id" in inp:
        fid = inp["id"]
        if not fid or len(fid) > 64 or not all(c.isalnum() or c in "-_." for c in fid):
            raise Refused("invalid_form_id", f"“{fid}” bir form kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı.", "id")
        if fid in taken:
            raise Refused("form_exists", f"“{fid}” kimlikli başka bir form var. Başka bir kimlik verin.", "id")
    else:
        fid = item_id_of(name if any(c.isalnum() for c in name) else "form", taken)
    if len(taken) >= 100:
        raise Refused("too_many_forms", "Projede en çok 100 form olur. Kullanmadığınız birini silin.", "id")
    lfields = node.get("fields", [])
    mapping = inp.get("map", {})
    for frm, to in sorted(mapping.items()):
        if not any(key(f) == frm for f in p["fields"]):
            raise Refused("invalid_map", f"Eşlemedeki “{frm}” paketin bir alanı değil.", f"map/{frm}")
        if not any(key(f) == to for f in lfields):
            raise Refused("field_not_found", f"“{node['name']}” katmanında “{to}” alanı yok.", f"map/{frm}")
    bound, pairs, missing = [], [], []
    for pf in p["fields"]:
        if key(pf) in mapping:
            tf = next(f for f in lfields if key(f) == mapping[key(pf)])
        else:
            tf = next((f for f in lfields if key(f) == key(pf)), None) or next((f for f in lfields if f["name"] == pf["name"]),
                                                                              None)
        if tf is None:
            missing.append(pf)
            continue
        if tf["kind"] != pf["kind"]:
            label = lambda f: f.get("alias", f["name"])  # noqa: E731
            raise Refused("field_kind_mismatch", f"Paketin “{label(pf)}” alanı {KINDS[pf['kind']]}; katmanın “{label(tf)}” "
                          f"alanı {KINDS[tf['kind']]}. Aynı türde bir alana bağlayın.", f"map/{key(pf)}")
        bound.append({"from": key(pf), "to": key(tf)})
        pairs.append((key(pf), pf["name"], key(tf), tf["name"]))
    if missing and not inp.get("addFields"):
        said = ", ".join(f.get("alias", f["name"]) for f in missing)
        raise Refused("form_fields_missing", f"Paketin şu alanları “{node['name']}” katmanında yok: {said}. map ile bağlayın ya da "
                      "addFields ile ekleyin.", "package.fields")
    domains = copy.deepcopy(doc["settings"].get("domains", []))
    added_domains, added, warnings = [], [], []
    for pf in missing:
        f = copy.deepcopy(pf)
        did = f.get("domain")
        if did:
            have = next((d for d in domains if d["id"] == did), None)
            given = next((d for d in p.get("domains", []) if d["id"] == did), None)
            if have:
                if given != have:
                    warnings.append({"code": "domain_kept", "message": f"Projede “{have['name']}” değer listesi vardı; "
                                     f"“{f.get('alias', f['name'])}” alanı projeninkine bağlandı.", "path": "package.domains"})
                f = {**{k: v for k, v in f.items() if k not in ("values", "min", "max", "domain")},
                     **{k: copy.deepcopy(have[k]) for k in ("values", "min", "max") if k in have}, "domain": did}
            elif given:
                domains.append(copy.deepcopy(given))
                added_domains.append(did)
            else:
                f.pop("domain")
        bound.append({"from": key(pf), "to": key(f)})
        added.append(f)
    body = rebind(p["body"], pairs, p["fields"])
    form = {"id": fid, "name": name.strip(), "layer": inp["layer"], "draft": body}
    fields_after = lfields + added if added else None
    return form, bound, [key(f) for f in added], added_domains, fields_after, domains if added_domains else None, warnings


def export_case(name, inp, op="execute", doc=None):
    own = doc is not None
    doc = doc or setup()
    try:
        out = package_of(doc, inp["form"], inp.get("draft", False))
    except Refused as r:
        return {"name": name, **({"setup": doc} if own else {}),
                "steps": [{"op": op, "input": inp, "result": fc.failed(r.code, r.message, r.path), "expect": UNCHANGED}]}
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
    return {"name": name, **({"setup": doc} if own else {}),
            "steps": [{"op": op, "input": inp, "result": result, "expect": UNCHANGED}]}


def import_step(doc, inp, op="execute"):
    try:
        form, bound, added, added_domains, fields_after, domains_after, warnings = imported(doc, inp)
    except Refused as r:
        return {"op": op, "input": inp, "result": fc.failed(r.code, r.message, r.path), "expect": UNCHANGED}, doc
    out = {"id": form["id"], "bound": bound, "added": added, "domains": added_domains, "errors": 0, "revision": "$current"}
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": UNCHANGED}, doc
    if op == "plan":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
                "expect": UNCHANGED}, doc
    after = copy.deepcopy(doc)
    after["settings"]["forms"].append(form)
    if fields_after is not None:
        fu.find(after["layers"], inp["layer"])["fields"] = fields_after
    if domains_after is not None:
        after["settings"]["domains"] = domains_after
    stepped = fields_after is not None
    # The form is a project setting: an edit that moves the revision, no undo step (docs/adr/0401 §9).
    expect = {"canUndo": stepped, "dirty": True, "revision": "changed",
              "settings": {"forms": after["settings"]["forms"], "domains": after["settings"]["domains"] or None},
              "layers": after["layers"]}
    return {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
            "expect": expect}, after


def import_case(name, inp, op="execute", doc=None):
    s, _ = import_step(doc or setup(), inp, op)
    return {"name": name, **({"setup": doc} if doc is not None else {}), "steps": [s]}


def export_cases():
    doc = setup()
    many = setup()
    many["settings"]["domains"] = []
    for f in fu.find(many["layers"], "bina")["fields"]:
        f.pop("domain", None)
    return [
        export_case("son yayın: bağladığı ve okuduğu alanlar katmanın sırasıyla, değer listesiyle", {"form": "denetim"}),
        export_case("taslak istenince taslağın gövdesi", {"form": "denetim", "draft": True}),
        export_case("eski yayın değil son yayın", {"form": "denetim"}, doc=doc),
        export_case("kuralında okunan bağlanmamış alan da pakette", {"form": "okur"}),
        export_case("iş akışının durum ve gerekçe alanları pakette", {"form": "akis"}),
        export_case("iş akışlı formun taslağı", {"form": "akis", "draft": True}),
        export_case("alanı olmayan form: boş alan listesi", {"form": "notlu"}),
        export_case("yayınlanmamış formun taslağı", {"form": "taslak", "draft": True}),
        export_case("yayınlanmamış form taslaksız reddedilir", {"form": "taslak"}),
        export_case("olmayan form", {"form": "yok"}),
        export_case("olmayan form taslakla da", {"form": "yok", "draft": True}),
        export_case("değer listesi olmayan projede liste taşınmaz", {"form": "denetim"}, doc=many),
        export_case("plan aynı paketi verir", {"form": "denetim"}, op="plan"),
        export_case("plan taslakla", {"form": "okur", "draft": True}, op="plan"),
        export_case("doğrulama hiçbir şey yazmaz", {"form": "denetim"}, op="validate"),
        export_case("doğrulama yayınsız formu reddeder", {"form": "taslak"}, op="validate"),
        export_case("notlu formun taslağı", {"form": "notlu", "draft": True}),
        export_case("okuyan formun planı reddi yok", {"form": "okur"}, op="plan"),
        export_case("akışın planı", {"form": "akis"}, op="plan"),
        export_case("olmayan formun planı", {"form": "yok"}, op="plan"),
    ]


def import_cases():
    doc = setup()
    pkg = package_of(doc, "denetim")
    akis = package_of(doc, "akis")
    okur = package_of(doc, "okur")
    mapped = {"Durum": "Durumu", "kat-sayisi": "KatSayisi", "Malzeme": "Malzemesi", "Not": "Açıklama"}
    foreign = copy.deepcopy(pkg)
    foreign["domains"] = [{"id": "malzeme2", "name": "Başka malzeme", "kind": "text", "values": [ch("ce", "Çelik")]}]
    for f in foreign["fields"]:
        if f["name"] == "Malzeme":
            f.update(domain="malzeme2", values=[ch("ce", "Çelik")])
    changed_list = copy.deepcopy(pkg)
    changed_list["domains"][0]["values"].append(ch("ce", "Çelik"))
    for f in changed_list["fields"]:
        if f["name"] == "Malzeme":
            f["values"].append(ch("ce", "Çelik"))
    wrong = {**copy.deepcopy(pkg), "format": "kentos.form"}
    old = {**copy.deepcopy(pkg), "version": 2}
    holey = copy.deepcopy(pkg)
    holey["fields"] = [f for f in holey["fields"] if f["name"] != "Not"]
    many = setup(FORMS + [{"id": f"f{k}", "name": f"Form {k}", "layer": "bina", "draft": {"revision": 1, "items": []}}
                          for k in range(100 - len(FORMS))])
    cases = [
        import_case("aynı katmana: alanlar aynı anahtarla bağlanır, kimlik adından", {"package": pkg, "layer": "bina"}),
        import_case("başka katmana eşlemeyle: anahtarlar ve kurallardaki adlar yeniden yazılır",
                    {"package": pkg, "layer": "yapi", "map": mapped, "id": "yapi-denetim"}),
        import_case("eşlemesiz başka katman: eksik alanlar reddedilir", {"package": pkg, "layer": "yapi"}),
        import_case("boş katmana alanlarıyla: alanlar eklenir, değer listesi projede", {"package": pkg, "layer": "bos",
                                                                                         "addFields": True}),
        import_case("boş katmana: projede olmayan liste de eklenir", {"package": foreign, "layer": "bos", "addFields": True}),
        import_case("projede aynı kimlikli farklı liste: projeninki kullanılır, uyarılır",
                    {"package": changed_list, "layer": "bos", "addFields": True}),
        import_case("eşleme kısmen, kalanlar eklenir", {"package": pkg, "layer": "yapi", "map": {"Durum": "Durumu"},
                                                        "addFields": True}),
        import_case("iş akışı yeniden bağlanır", {"package": akis, "layer": "bos", "addFields": True}),
        import_case("okunan alan da bağlanır", {"package": okur, "layer": "yapi", "map": {"Durum": "Durumu", "kat-sayisi": "KatSayisi"}}),
        import_case("ad verilir", {"package": pkg, "layer": "bina", "name": "Denetim kopyası"}),
        import_case("başka türde alana bağlanamaz (aynı adla)", {"package": pkg, "layer": "kat", "addFields": True}),
        import_case("eşlemenin anahtarı paketin değil", {"package": pkg, "layer": "yapi", "map": {"Yok": "Durumu"}}),
        import_case("eşlemenin hedefi katmanda yok", {"package": pkg, "layer": "yapi", "map": {"Durum": "Yok"}}),
        import_case("eşleme başka türe", {"package": pkg, "layer": "yapi", "map": {"Durum": "KatSayisi"}}),
        import_case("olmayan katman", {"package": pkg, "layer": "yok"}),
        import_case("grup katman değil", {"package": pkg, "layer": "grup"}),
        import_case("başka biçim", {"package": wrong, "layer": "bina"}),
        import_case("başka sürüm", {"package": old, "layer": "bina"}),
        import_case("formun andığı alan pakette yok", {"package": holey, "layer": "bina"}),
        import_case("kimlik zaten var", {"package": pkg, "layer": "bina", "id": "denetim"}),
        import_case("geçersiz kimlik", {"package": pkg, "layer": "bina", "id": "bir kimlik"}),
        import_case("boş ad", {"package": pkg, "layer": "bina", "name": "  "}),
        import_case("100 form varken", {"package": pkg, "layer": "bina"}, doc=many),
        import_case("plan yazmaz", {"package": pkg, "layer": "bos", "addFields": True}, op="plan"),
        import_case("doğrulama yazmaz", {"package": pkg, "layer": "bina"}, op="validate"),
        import_case("geçersiz sürüm", {"package": pkg, "layer": "bina", "expectedRevision": "07"}),
    ]
    cases[-1]["steps"][0].update(result=fc.failed("invalid_revision", fc.REVISION, "expectedRevision"), expect=UNCHANGED)
    conflict = import_case("sürüm çakışması", {"package": pkg, "layer": "bina", "expectedRevision": "999"})
    conflict["steps"][0].update(result=fc.failed("revision_conflict", fc.CONFLICT, "expectedRevision", "conflict"),
                                expect=UNCHANGED)
    cases.append(conflict)
    # Undo takes the fields and the lists back; the form stays, a project setting.
    a, _ = import_step(doc, {"package": foreign, "layer": "bos", "addFields": True})
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": doc["layers"],
                                     "settings": {"domains": doc["settings"]["domains"]}}}
    cases.append({"name": "geri alma alanları ve listeyi geri verir, form ayarda kalır", "steps": [a, undo]})
    return cases


def build(command, title, cases, note):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": f"ADR 0435: {note} Written by scripts/fixtures/forms_package_cases.py without KentOS code; do not edit "
                    "by hand.", "setup": setup(), "cases": cases}


def main() -> int:
    files = {
        "cad.forms.export.json": build("cad.forms.export", "Formu dışa aktar", export_cases(), "formun paketi."),
        "cad.forms.import.json": build("cad.forms.import", "Formu içe aktar", import_cases(), "paketin içe alınması."),
    }
    check = "--check" in sys.argv[1:]
    bad = False
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        n = len(data["cases"])
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
                bad = True
            else:
                print(f"{path.relative_to(ROOT)} güncel ({n} durum)")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)} yazıldı ({n} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
