"""Writes the shared cases of the data profiles (docs/adr/0541; plan/gis.md CBS-34):

- fixtures/commands/v1/cad.profiles.apply.json (`cad.profiles.apply` v1: the profile's missing parts added)
- fixtures/commands/v1/cad.profiles.check.json (`cad.profiles.check` v1: where the data departs from the profile)
- fixtures/profiles/v1/belediye.json (the reference municipality project: districts, blocks, parcels, buildings, roads,
  addresses; no personal data) and fixtures/profiles/v1/links.json (its relations' links, written by `cad.entities.set`)

The rules are written here again from the ADR and the contract (crates/shared/commands/src/cad_profiles.rs), the tree,
the objects and the settings as plain JSON, without KentOS code: the layer path found by Turkish-folded names, the
fields added with their value lists, the settings added, the findings in their order with their messages. A field's
value is checked by layer_field_cases.py's `check`; a polygon's area is the shoelace sum, holes taken off. The built-in
profile is read from catalog/v1/profiles/ as data. The desktop (crates/native/application/tests/all/fixtures.rs) and
Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/profile_cases.py           # writes the files
    python3 scripts/fixtures/profile_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
REF = ROOT / "fixtures/profiles/v1"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


T = _load("layer_tree_command_cases")
FIELDS = T.FIELDS
node, step, completed, failed, VALID = T.node, T.step, T.completed, T.failed, T.VALID
INVALID_REVISION, CONFLICT = T.INVALID_REVISION, T.CONFLICT

PROFILE = json.loads((ROOT / "catalog/v1/profiles/tucbs-temel.json").read_text(encoding="utf-8"))
NEW_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}
X, Y = 500000, 4400000
STEP_NAME = "Veri profili"
KIND_LABEL = {"text": "Metin", "integer": "Tam sayı", "decimal": "Ondalık sayı", "date": "Tarih",
              "boolean": "Evet/hayır", "datetime": "Tarih ve saat", "time": "Saat"}
GEOMETRY = {"area": ("alan", {"polygon"}), "line": ("çizgi", {"line", "polyline"}), "point": ("nokta", {"point"}),
            "record": ("kayıt", {"record"})}


# ── Words ────────────────────────────────────────────────────────────────────

def fold_name(name):
    """Lower case as Turkish writes it: I ı, İ i."""
    out = []
    for c in name:
        out.append("ı" if c == "I" else "i" if c == "İ" else c.lower())
    return "".join(out)


def m2(x):
    return f"{x:.2f}".replace(".", ",")


def kind_word(f):
    return KIND_LABEL[f["kind"]].lower()


def path_words(path):
    return " › ".join(path)


# ── The tree ─────────────────────────────────────────────────────────────────

def find(nodes, path):
    """("layer", id) | ("blocked", id) | ("missing", parent id or None, depth)."""
    here, parent = nodes, None
    for i, name in enumerate(path):
        last = i + 1 == len(path)
        n = next((n for n in here if fold_name(n["name"].strip()) == fold_name(name.strip())), None)
        if n is None:
            return ("missing", parent, i)
        if n["type"] == "layer" and last:
            return ("layer", n["id"])
        if n["type"] == "group" and not last:
            parent, here = n["id"], n["children"]
            continue
        return ("blocked", n["id"])
    return ("missing", parent, len(path))


def get(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = get(n["children"], id_)
        if hit is not None:
            return hit
    return None


def layer_field(fields, name):
    hit = next((f for f in fields if f["name"] == name), None)
    return hit or next((f for f in fields if f.get("id", f["name"]) == name), None)


def key_of(f):
    return f.get("id", f["name"])


def applied(field, domains):
    """docs/adr/0430: a field linked to a list takes its rules (values, range, unit)."""
    d = next((d for d in domains if d["id"] == field.get("domain")), None)
    if d is None or d["kind"] != field["kind"]:
        return copy.deepcopy(field)
    out = {k: v for k, v in field.items() if k not in ("values", "min", "max", "minExclusive", "maxExclusive", "unit")}
    for k in ("values", "min", "max", "unit"):
        if k in d:
            out[k] = copy.deepcopy(d[k])
    for k in ("minExclusive", "maxExclusive"):
        if d.get(k):
            out[k] = True
    out["domain"] = d["id"]
    return out


def follow(fields, domains):
    return [applied(f, domains) for f in fields]


# ── Apply ────────────────────────────────────────────────────────────────────

def apply(doc, profile):
    """The profile's missing parts written to `doc` (changed in place); (output, warnings, fresh layer names)."""
    out = {"changed": False, "layers": [], "fields": [], "domains": [], "relations": [], "address": False,
           "revision": "$current"}
    warnings = []
    settings = doc["settings"]
    domains = copy.deepcopy(settings.get("domains", []))
    for d in profile.get("domains", []):
        have = next((x for x in domains if x["id"] == d["id"]), None)
        if have is not None:
            if have["kind"] != d["kind"] or have.get("values") != d.get("values"):
                warnings.append({"code": "profile_domain_differs",
                                 "message": f"Projenin “{have['name']}” değer listesi profilinkinden farklı; projeninki korundu."})
        else:
            domains.append(copy.deepcopy(d))
            out["domains"].append(d["id"])
    fresh = 0
    layer_of = {}
    for c in profile["classes"]:
        fields = follow(c["fields"], domains)
        place = find(doc["layers"], c["layer"])
        if place[0] == "layer":
            n = get(doc["layers"], place[1])
            added = []
            for f in fields:
                h = layer_field(n.get("fields", []), f["name"])
                if h is not None and h["kind"] != f["kind"]:
                    warnings.append({"code": "profile_field_differs",
                                     "message": f"“{n['name']}” katmanının “{h['name']}” alanı {kind_word(h)}, profilde {kind_word(f)}; katmanınki korundu."})
                elif h is None:
                    added.append(copy.deepcopy(f))
            layer_of[c["id"]] = n["id"]
            if not added:
                continue
            if n["locked"]:
                warnings.append({"code": "profile_layer_skipped",
                                 "message": f"“{n['name']}” katmanı kilitli; profilin alanları eklenmedi."})
                continue
            out["fields"] += [f"{n['name']}.{f['name']}" for f in added]
            n["fields"] = n.get("fields", []) + added
        elif place[0] == "blocked":
            name = get(doc["layers"], place[1])["name"]
            warnings.append({"code": "profile_layer_skipped",
                             "message": f"“{c['title']}” sınıfının yolunda ({path_words(c['layer'])}) “{name}” var ama katman değil ya da grup değil; sınıf atlandı."})
        else:
            _, parent, depth = place
            for i in range(depth, len(c["layer"])):
                fresh += 1
                id_ = f"$layer:{fresh}"
                last = i + 1 == len(c["layer"])
                if last:
                    new = node(id_, c["layer"][i].strip(), style={**NEW_STYLE, "color": c["color"]}, fields=copy.deepcopy(fields))
                else:
                    new = node(id_, c["layer"][i].strip(), "group", style=NEW_STYLE)
                into = doc["layers"] if parent is None else get(doc["layers"], parent)["children"]
                into.append(new)
                out["layers"].append(id_)
                parent = id_
            layer_of[c["id"]] = parent
    changed_settings = bool(out["domains"])
    settings["domains"] = domains
    if not settings["domains"]:
        settings.pop("domains")
    relations = copy.deepcopy(settings.get("relations", []))
    for r in profile.get("relations", []):
        if r["parent"] not in layer_of or r["child"] not in layer_of:
            continue
        parent, child = layer_of[r["parent"]], layer_of[r["child"]]
        h = layer_field(get(doc["layers"], child).get("fields", []), r["field"])
        key = key_of(h) if h else r["field"]
        if any(x["id"] == r["id"] or (x["parent"] == parent and x["child"] == child and x["field"] == key) for x in relations):
            continue
        relations.append({"id": r["id"], "name": r["name"], "parent": parent, "child": child, "field": key})
        out["relations"].append(r["id"])
    if out["relations"]:
        settings["relations"] = relations
        changed_settings = True
    a = profile.get("address")
    if a is not None:
        loc = locator(profile, layer_of)
        have = settings.get("addressLocators", [])
        if loc is not None and not any(x["id"] == loc["id"] or x["name"] == loc["name"] for x in have):
            settings["addressLocators"] = have + [loc]
            out["address"] = True
            changed_settings = True
    out["changed"] = changed_settings or bool(out["layers"]) or bool(out["fields"])
    return out, warnings


def locator(profile, layer_of):
    a = profile["address"]
    try:
        loc = {"id": f"profil-{profile['name']}", "name": profile["title"]}
        if "doors" in a:
            loc["doors"] = {**a["doors"], "layer": layer_of[a["doors"]["layer"]]}
        if "streets" in a:
            loc["streets"] = {**a["streets"], "layer": layer_of[a["streets"]["layer"]]}
        if a.get("areas"):
            loc["areas"] = [{**x, "layer": layer_of[x["layer"]]} for x in a["areas"]]
    except KeyError:
        return None
    loc["minScore"] = a["minScore"]
    loc["reach"] = float(a["reach"])
    return loc


# ── Check ────────────────────────────────────────────────────────────────────

def area_of(e):
    def ring(pts):
        s = 0.0
        for i, p in enumerate(pts):
            q = pts[(i + 1) % len(pts)]
            s += p["x"] * q["y"] - q["x"] * p["y"]
        return abs(s) / 2
    return ring(e["pts"]) - sum(ring(h["pts"]) for h in e.get("holes", []))


def value_of(e, key):
    v = e.get("attrs", {}).get(key)
    if v is None or FIELDS.trim(v) == "":
        return None
    return FIELDS.trim(v)


def finding(kind, cls, layer, subject, uids, fixable, message, **extra):
    f = {"kind": kind}
    if cls is not None:
        f["class"] = cls
    if layer is not None:
        f["layer"] = layer
    if subject is not None:
        f["subject"] = subject
    f.update({k: v for k, v in extra.items() if k == "value"})
    f["uids"] = list(uids)
    f["count"] = len(uids)
    f.update({k: v for k, v in extra.items() if k in ("geometric", "registered")})
    f["fixable"] = fixable
    f["message"] = message
    return f


def grouped(rows):
    order, by = [], {}
    for value, uid in rows:
        if value not in by:
            order.append(value)
            by[value] = []
        by[value].append(uid)
    return [(v, by[v]) for v in order]


def uid(e):
    return f"$uidOf:{e['id']}"


def report(doc, profile):
    findings = []
    settings = doc["settings"]
    for d in profile.get("domains", []):
        if not any(x["id"] == d["id"] for x in settings.get("domains", [])):
            findings.append(finding("missingDomain", None, None, d["id"], [], True,
                                    f"“{d['name']}” değer listesi projede yok; Veri profilini uygula onu ekler."))
    unit = settings.get("drawingUnit", "m")
    per = {"m": 1.0, "cm": 0.01, "mm": 0.001}[unit]
    square = per * per
    summaries, layer_of = [], {}
    for c in profile["classes"]:
        start = len(findings)
        place = find(doc["layers"], c["layer"])
        layer, objects = None, 0
        if place[0] == "layer":
            layer = place[1]
            layer_of[c["id"]] = layer
            objects = class_findings(doc, profile, c, layer, square, findings)
        elif place[0] == "blocked":
            name = get(doc["layers"], place[1])["name"]
            findings.append(finding("missingLayer", c["id"], None, None, [], False,
                                    f"“{c['title']}” sınıfının yolunda ({path_words(c['layer'])}) “{name}” var ama katman değil ya da grup değil."))
        else:
            findings.append(finding("missingLayer", c["id"], None, None, [], True,
                                    f"“{c['title']}” sınıfının katmanı ({path_words(c['layer'])}) çizimde yok; Veri profilini uygula onu ekler."))
        s = {"class": c["id"], "title": c["title"]}
        if layer is not None:
            s["layer"] = layer
        s.update({"objects": objects, "findings": len(findings) - start})
        summaries.append(s)
    title = {c["id"]: c["title"] for c in profile["classes"]}
    for r in profile.get("relations", []):
        if r["parent"] not in layer_of or r["child"] not in layer_of:
            continue
        parent, child = layer_of[r["parent"]], layer_of[r["child"]]
        h = layer_field(get(doc["layers"], child).get("fields", []), r["field"])
        key = key_of(h) if h else r["field"]
        if not any(x["id"] == r["id"] or (x["parent"] == parent and x["child"] == child and x["field"] == key)
                   for x in settings.get("relations", [])):
            findings.append(finding("missingRelation", None, None, r["id"], [], True,
                                    f"“{r['name']}” ilişkisi projede yok; Veri profilini uygula onu ekler."))
        parents = {e["id"] for e in doc["entities"] if e["layerId"] == parent}
        links = doc.get("_links", {})
        broken = []
        for e in doc["entities"]:
            if e["layerId"] != child:
                continue
            v = links.get((e["id"], key))
            if v is not None:
                if v not in parents:
                    broken.append(uid(e))
                continue
            v = value_of(e, key)
            if v is not None:
                broken.append(uid(e))
        if broken:
            findings.append(finding("relationBroken", r["child"], child, r["id"], broken, False,
                                    f"“{r['name']}” ilişkisinde {len(broken)} {title[r['child']]} nesnesinin “{r['field']}” alanı {title[r['parent']]} katmanında olmayan bir nesneyi anıyor."))
            next(s for s in summaries if s["class"] == r["child"])["findings"] += 1
    if profile.get("address") is not None:
        id_ = f"profil-{profile['name']}"
        if not any(x["id"] == id_ or x["name"] == profile["title"] for x in settings.get("addressLocators", [])):
            findings.append(finding("missingAddress", None, None, id_, [], True,
                                    "Profilin adres verisi projede yok; Veri profilini uygula onu ekler."))
    return {"profile": profile["name"], "title": profile["title"], "version": profile["version"],
            "basis": profile["basis"], "classes": summaries, "findings": findings, "clean": not findings}


def class_findings(doc, profile, c, layer, square, findings):
    n = get(doc["layers"], layer)
    name = n["name"]
    fields = follow(c["fields"], profile.get("domains", []))
    keys = []
    for f in fields:
        h = layer_field(n.get("fields", []), f["name"])
        if h is None:
            findings.append(finding("missingField", c["id"], layer, f["name"], [], True,
                                    f"“{name}” katmanında “{f['name']}” alanı yok; Veri profilini uygula onu ekler."))
            continue
        if h["kind"] != f["kind"]:
            findings.append(finding("fieldKind", c["id"], layer, f["name"], [], False,
                                    f"“{name}” katmanının “{h['name']}” alanı {kind_word(h)}, profilde {kind_word(f)}."))
        keys.append((f, key_of(h)))
    objects = [e for e in doc["entities"] if e["layerId"] == layer]
    word, kinds = GEOMETRY[c["geometry"]]
    wrong = [uid(e) for e in objects if e["kind"] not in kinds]
    if wrong:
        findings.append(finding("wrongGeometry", c["id"], layer, None, wrong, False,
                                f"“{name}” katmanında {len(wrong)} nesne {word} değil."))
    for f, key in keys:
        if f.get("required"):
            empty = [uid(e) for e in objects if value_of(e, key) is None and doc.get("_links", {}).get((e["id"], key)) is None]
            if empty:
                findings.append(finding("requiredEmpty", c["id"], layer, f["name"], empty, False,
                                        f"“{name}” katmanında {len(empty)} nesnenin “{f['name']}” alanı boş; alan zorunlu."))
        bad, why = [], {}
        for e in objects:
            v = value_of(e, key)
            if v is None:
                continue
            r = FIELDS.check(f, v)
            if "error" in r:
                why.setdefault(v, r["message"])
                bad.append((v, uid(e)))
        for v, uids in grouped(bad):
            findings.append(finding("badValue", c["id"], layer, f["name"], uids, False,
                                    f"“{name}” katmanında {len(uids)} nesnenin “{f['name']}” değeri “{v}” kurala uymuyor: {why[v]}",
                                    value=v))
    if c.get("key"):
        key = next((k for f, k in keys if f["name"] == c["key"]), None)
        if key is not None:
            rows = [(value_of(e, key), uid(e)) for e in objects if value_of(e, key) is not None]
            for v, uids in grouped(rows):
                if len(uids) > 1:
                    findings.append(finding("duplicateKey", c["id"], layer, c["key"], uids, False,
                                            f"“{name}” katmanında “{v}” kimliği {len(uids)} nesnede var; “{c['key']}” tekil olmalı.",
                                            value=v))
    ra = c.get("registeredArea")
    if ra:
        hit = next(((f, k) for f, k in keys if f["name"] == ra["field"]), None)
        ident = next((k for f, k in keys if f["name"] == c.get("key")), None)
        if hit is not None:
            f, key = hit
            for e in objects:
                if e["kind"] not in kinds:
                    continue
                v = value_of(e, key)
                if v is None:
                    continue
                r = FIELDS.check(f, v)
                if "error" in r:
                    continue
                registered = float(r["value"])
                geometric = area_of(e) * square
                diff = geometric - registered
                if abs(diff) <= ra["tolerance"]:
                    continue
                who = f"“{value_of(e, ident)}” nesnesinin" if ident and value_of(e, ident) else "Nesnenin"
                findings.append(finding("areaDifference", c["id"], layer, ra["field"], [uid(e)], False,
                                        f"{who} geometrik alanı {m2(geometric)} m², kayıtlı alanı {m2(registered)} m²; fark {m2(diff)} m² profilin toleransını ({m2(ra['tolerance'])} m²) aşıyor. Hiçbiri öbürünün yerine yazılmaz.",
                                        geometric=geometric, registered=registered))
    return len(objects)


# ── The reference municipality ───────────────────────────────────────────────

def rect(x, y, w, h):
    return [{"x": X + x, "y": Y + y}, {"x": X + x + w, "y": Y + y}, {"x": X + x + w, "y": Y + y + h}, {"x": X + x, "y": Y + y + h}]


def polygon(id_, layer, pts, attrs, holes=None):
    e = {"kind": "polygon", "id": id_, "layerId": layer, "attrs": attrs, "pts": pts}
    if holes:
        e["holes"] = [{"pts": h} for h in holes]
    return e


def polyline(id_, layer, pts, attrs):
    return {"kind": "polyline", "id": id_, "layerId": layer, "attrs": attrs, "pts": [{"x": X + x, "y": Y + y} for x, y in pts]}


def point(id_, layer, x, y, attrs):
    return {"kind": "point", "id": id_, "layerId": layer, "attrs": attrs, "p": {"x": X + x, "y": Y + y}}


def cls(id_):
    return next(c for c in PROFILE["classes"] if c["id"] == id_)


def domain(id_):
    return next(d for d in PROFILE["domains"] if d["id"] == id_)


# The project's own list of building uses, which differs from the profile's (no “sanayi”).
OWN_USES = {**domain("bina-kullanim"), "values": [v for v in domain("bina-kullanim")["values"] if v["code"] != "sanayi"]}
PROJECT_DOMAINS = [domain("parsel-nitelik"), OWN_USES]
BOGUS = "00000000-0000-7000-8000-000000000000"


def reference_layers():
    mahalle = follow(cls("mahalle")["fields"], PROJECT_DOMAINS)
    ada = follow(cls("ada")["fields"], PROJECT_DOMAINS)
    parsel = follow(cls("parsel")["fields"], PROJECT_DOMAINS)
    # The buildings' layer has no Kullanim yet.
    bina = [f for f in follow(cls("bina")["fields"], PROJECT_DOMAINS) if f["name"] != "Kullanim"]
    # The roads' Tur is a plain text field: no list of its own.
    yol = [({k: v for k, v in f.items() if k != "domain"} if f["name"] == "Tur" else f) for f in cls("yol")["fields"]]
    kapi = follow(cls("adres")["fields"], PROJECT_DOMAINS)
    return [
        node("idari", "İdari birimler", "group", [node("mahalle", "Mahalle", style={**NEW_STYLE, "color": "#8A94A6"}, fields=mahalle)]),
        node("kadastro", "Kadastro", "group", [
            node("ada", "Ada", style={**NEW_STYLE, "color": "#C8A165"}, fields=ada),
            node("parsel", "Parsel", style={**NEW_STYLE, "color": "#E5484D"}, fields=parsel),
        ]),
        node("binalar", "Binalar", "group", [node("bina", "Bina", style={**NEW_STYLE, "color": "#0090FF"}, fields=bina)]),
        node("ulasim", "Ulaşım", "group", [node("yol", "Yol orta hattı", style={**NEW_STYLE, "color": "#F5A524"}, fields=yol)]),
        node("adresler", "Adres", "group", [node("kapi", "Kapı", style={**NEW_STYLE, "color": "#30A46C"}, fields=kapi)]),
    ]


def reference_entities():
    def parcel(id_, x, w, no, tapu, nitelik="arsa", ada="101", ident=None, holes=None, y=10, h=40):
        return polygon(id_, "parsel", rect(x, y, w, h),
                       {"ParselKimlik": ident or f"{ada}/{no}", "AdaNo": ada, "ParselNo": no, "TapuAlani": tapu, "Nitelik": nitelik},
                       holes)
    place = {"Mahalle": "Cumhuriyet", "Ilce": "Merkez", "Il": "Sivas"}
    return [
        polygon(1, "mahalle", rect(0, 0, 200, 120), {"MahalleKodu": "MH-01", "Ad": "Cumhuriyet", "Ilce": "Merkez", "Il": "Sivas"}),
        polygon(2, "ada", rect(10, 10, 80, 40), {"AdaKimlik": "MH-01/101", "AdaNo": "101", "MahalleKodu": "MH-01"}),
        polygon(3, "ada", rect(110, 10, 80, 40), {"AdaKimlik": "MH-01/102", "AdaNo": "102", "MahalleKodu": "MH-01"}),
        parcel(4, 10, 20, "1", "800.00"),
        parcel(5, 30, 20, "2", "799.50"),
        parcel(6, 50, 20, "3", "812.40"),
        parcel(7, 70, 20.5, "4", "820", nitelik="konut"),
        parcel(8, 110, 40, "1", "1500", ada="102", holes=[rect(10 + 110, 10 + 10, 10, 10)]),
        parcel(9, 150, 40, "", "1600", ada="102", ident="101/1"),
        polygon(10, "bina", rect(12, 15, 16, 20), {"BinaKimlik": "B-1", "KatSayisi": "4", "Kullanim": "konut"}),
        polygon(11, "bina", rect(32, 15, 16, 20), {"BinaKimlik": "B-2", "KatSayisi": "250", "Kullanim": "ticaret"}),
        polygon(12, "bina", rect(115, 30, 20, 15), {"BinaKimlik": "B-3", "KatSayisi": "2", "Parsel": BOGUS}),
        polygon(13, "bina", rect(52, 15, 16, 20), {"BinaKimlik": "B-4", "KatSayisi": "3"}),
        point(14, "bina", 80, 30, {"BinaKimlik": "B-5"}),
        polyline(15, "yol", [(0, 5), (200, 5)], {"YolKimlik": "Y-1", "Ad": "Atatürk Caddesi", "Tur": "cadde", "Mahalle": "Cumhuriyet"}),
        polyline(16, "yol", [(100, 0), (100, 120)], {"YolKimlik": "Y-2", "Ad": "İstasyon Sokak", "Tur": "otoyol", "Mahalle": "Cumhuriyet"}),
        point(17, "kapi", 20, 12, {"AdresKodu": "1000000001", "Yol": "Atatürk Caddesi", "KapiNo": "12", **place, "PostaKodu": "58040"}),
        point(18, "kapi", 40, 12, {"AdresKodu": "1000000002", "Yol": "Atatürk Caddesi", "KapiNo": "14", **place, "PostaKodu": "58040"}),
        point(19, "kapi", 130, 12, {"AdresKodu": "1000000003", "Yol": "Atatürk Caddesi", "KapiNo": " ", **place, "PostaKodu": "58040"}),
        point(20, "kapi", 170, 12, {"AdresKodu": "1000000004", "Yol": "Atatürk Caddesi", "KapiNo": "5/A", **place, "PostaKodu": "580400"}),
    ]


# (child id, field, parent id): written by `cad.entities.set` with the parent's persistent id.
LINKS = [(10, "Parsel", 4), (11, "Parsel", 5), (13, "Parsel", 6), (17, "Bina", 10), (18, "Bina", 11), (19, "Bina", 12)]


def reference(layers=None, entities=None, unit=None, **settings):
    s = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
         "workspace": "gis", "domains": copy.deepcopy(PROJECT_DOMAINS),
         "relations": [{"id": "parsel-bina", "name": "Parselin binaları", "parent": "parsel", "child": "bina", "field": "Parsel"}]}
    if unit:
        s["drawingUnit"] = unit
    s.update(copy.deepcopy(settings))
    s = {k: v for k, v in s.items() if v is not None}
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Belediye örnek projesi",
        "settings": s,
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(reference_layers() if layers is None else layers),
        "activeLayer": "parsel" if layers is None else first_layer(layers),
        "entities": copy.deepcopy(reference_entities() if entities is None else entities),
        "styles": {"items": [], "categories": []},
    }


def first_layer(nodes):
    for n in nodes:
        if n["type"] == "layer":
            return n["id"]
        hit = first_layer(n["children"])
        if hit:
            return hit
    return None


def link_steps(links=LINKS):
    out = []
    for child, field, parent in links:
        out.append(step("execute", {"uids": [f"$uidOf:{child}"], "operation": "attributes", "attrs": {field: f"$uidOf:{parent}"}},
                        completed({"changed": [f"$uidOf:{child}"], "ids": [child], "revision": "$current"}),
                        command="cad.entities.set"))
    return out


def linked(doc, links=LINKS):
    """The document as the links leave it, for the reference rules (`_links` keyed by (object, field))."""
    d = copy.deepcopy(doc)
    d["_links"] = {(c, f): p for c, f, p in links}
    for e in d["entities"]:
        for c, f, p in links:
            if e["id"] == c:
                e["attrs"].pop(f, None)
    return d


def tree_after(doc):
    return {"layers": doc["layers"]}


# ── Cases ────────────────────────────────────────────────────────────────────

APPLY, CHECK = "cad.profiles.apply", "cad.profiles.check"
NAME = {"name": "tucbs-temel"}
SMALL = {
    "format": "kentos.data-profile", "version": 1, "name": "peyzaj", "title": "Peyzaj profili",
    "description": "Ağaçlar ve yeşil alanlar.", "basis": "Deneme profili.",
    "domains": [{"id": "agac-tur", "name": "Ağaç türü", "kind": "text",
                 "values": [{"code": "cinar", "label": "Çınar"}, {"code": "mese", "label": "Meşe"}]}],
    "classes": [
        {"id": "agac", "title": "Ağaç", "theme": "Peyzaj", "layer": ["Peyzaj", "Ağaç"], "color": "#30A46C", "geometry": "point",
         "key": "AgacNo", "fields": [{"name": "AgacNo", "kind": "text", "required": True},
                                     {"name": "Tur", "alias": "Tür", "kind": "text", "domain": "agac-tur"}]},
        {"id": "yesil", "title": "Yeşil alan", "theme": "Peyzaj", "layer": ["Peyzaj", "Yeşil alan"], "color": "#46A758",
         "geometry": "area", "fields": [{"name": "Ad", "kind": "text"},
                                         {"name": "KayitliAlan", "alias": "Kayıtlı alan", "kind": "decimal", "scale": 2}],
         "registeredArea": {"field": "KayitliAlan", "tolerance": 0.5}},
    ],
}


def bad(profile, **change):
    p = copy.deepcopy(profile)
    p.update(change)
    return p


def bad_class(i, **change):
    p = copy.deepcopy(SMALL)
    p["classes"][i].update(change)
    return p


INVALID = [
    ("biçim yanlış", bad(SMALL, format="kentos.profile"), "Profilin biçimi “kentos.data-profile”, sürümü 1 olmalı."),
    ("sınıf iki kez", bad(SMALL, classes=[SMALL["classes"][0], SMALL["classes"][0]]), "“agac” sınıfı iki kez var."),
    ("sınıf kimliği büyük harf", bad_class(0, id="Agac"), "“Agac” sınıf kimliği olamaz: küçük harf, rakam ve tire, en çok 64."),
    ("kimlik alanı alanlarda yok", bad_class(0, key="Numara"), "“Ağaç” sınıfının kimlik alanı “Numara” alanlarında yok."),
    ("kayıtlı alan metin", bad_class(1, registeredArea={"field": "Ad", "tolerance": 0.5}), "“Yeşil alan” sınıfının kayıtlı alanı “Ad” sayı alanı olmalı."),
    ("değer listesi profilde yok", bad(SMALL, domains=[]), "“Ağaç” sınıfının “Tur” alanı profilde olmayan “agac-tur” değer listesini anıyor."),
    ("ilişki olmayan sınıfa", bad(SMALL, relations=[{"id": "r", "name": "Bağ", "parent": "agac", "child": "bina", "field": "Ad"}]),
     "“Bağ” ilişkisi profilde olmayan bir sınıfı anıyor."),
    ("ilişki alanı metin değil", bad(SMALL, relations=[{"id": "r", "name": "Bağ", "parent": "agac", "child": "yesil", "field": "KayitliAlan"}]),
     "“Bağ” ilişkisinin alanı “KayitliAlan”, “Yeşil alan” sınıfının metin alanı olmalı."),
]

UNKNOWN = "“yok” adlı veri profili yok; KentOS'un profili: tucbs-temel."
NONE_GIVEN = "Profili adıyla (name: tucbs-temel) ya da kendisini (profile) verin."


def empty_doc():
    return reference(layers=[node("cizim", "Çizim")], entities=[], domains=None, relations=None)


def apply_case(name, doc, input_, links=(), plan=True, undo=True, extra_steps=(), profile=PROFILE):
    before = linked(doc, list(links))
    after = copy.deepcopy(before)
    out, warnings = apply(after, profile)
    steps = list(link_steps(list(links))) + list(extra_steps)
    if plan:
        steps.append(step("plan", input_, completed(out, warnings), {"revision": "same", "layers": before["layers"]}))
    expect = {"layers": after["layers"], "settings": {k: after["settings"].get(k) for k in ("domains", "relations", "addressLocators")}}
    steps.append(step("execute", input_, completed(out, warnings), expect))
    if undo and (out["layers"] or out["fields"]):
        steps.append(step("undo", returns=STEP_NAME, expect={"layers": before["layers"],
                                                             "settings": expect["settings"]}))
    return {"name": name, "setup": doc, "steps": steps}, after, out


def apply_cases():
    cases = []
    ref = reference()
    c, after, _ = apply_case("Belediye örnek projesi: eksik katman, alan, değer listeleri, ilişki ve adres verisi eklenir; tek adım geri alınır",
                             ref, NAME, LINKS)
    cases.append(c)
    # Applied twice: the second finds nothing to add.
    again = copy.deepcopy(after)
    out2, w2 = apply(again, PROFILE)
    c2 = copy.deepcopy(c)
    c2["name"] = "İkinci kez uygulanınca eklenecek bir şey kalmaz (changed false)"
    c2["steps"] = [s for s in c2["steps"] if s["op"] != "undo"]
    c2["steps"].append(step("execute", NAME, completed(out2, w2), {"layers": after["layers"]}))
    cases.append(c2)
    cases.append(apply_case("Boş çizime bütün profil: gruplar, katmanlar alanlarıyla, dört değer listesi, iki ilişki, adres verisi",
                            empty_doc(), NAME)[0])
    # A field the layer has with another kind is kept.
    lay = reference_layers()
    for f in get(lay, "bina")["fields"]:
        if f["name"] == "KatSayisi":
            f.clear()
            f.update({"name": "KatSayisi", "alias": "Kat sayısı", "kind": "text", "length": 10})
    cases.append(apply_case("Türü farklı alan korunur: profile_field_differs", reference(layers=lay), NAME, LINKS)[0])
    # A locked layer gets no fields.
    lay = reference_layers()
    get(lay, "bina")["locked"] = True
    cases.append(apply_case("Kilitli katmana alan eklenmez: profile_layer_skipped", reference(layers=lay), NAME,
                            [l for l in LINKS if l[1] != "Parsel"])[0])
    # A layer named Kadastro where the group should be: its classes are left out.
    blocked = [node("cizim", "Çizim"), node("kadastro-katman", "KADASTRO")]
    cases.append(apply_case("Yolunda katman olan sınıflar atlanır, öbürleri eklenir; büyük harfli ad Türkçe katlanarak bulunur",
                            reference(layers=blocked, entities=[], domains=None, relations=None), NAME)[0])
    # The profile itself, given with a name too: the profile goes first.
    cases.append(apply_case("Profil kendisiyle verilir; ad da verilmişse profil önce gelir",
                            empty_doc(), {"name": "tucbs-temel", "profile": SMALL}, profile=SMALL)[0])
    # A relation of the same id and an address data of the same name are kept.
    own_rel = [{"id": "bina-adres", "name": "Adresler", "parent": "bina", "child": "kapi", "field": "AdresKodu"},
               {"id": "parsel-bina", "name": "Parselin binaları", "parent": "parsel", "child": "bina", "field": "Parsel"}]
    loc = {"id": "adresler", "name": "TUCBS temel belediye profili",
           "doors": {"layer": "kapi", "street": "Yol", "door": "KapiNo"}, "minScore": 60, "reach": 25.0}
    cases.append(apply_case("Aynı kimlikli ilişki ve aynı adlı adres verisi korunur",
                            reference(relations=own_rel, addressLocators=[loc]), NAME, LINKS)[0])
    # Validate.
    cases.append({"name": "Doğrulama: profil adıyla", "setup": ref, "steps": [step("validate", NAME, VALID)]})
    # Expected revision held.
    c, _, out = apply_case("Beklenen sürüm tutunca yazılır", ref, {**NAME, "expectedRevision": "$once"}, plan=False, undo=False)
    c["steps"].insert(0, step("captureRevision", **{"as": "once"}))
    cases.append(c)
    # Refusals.
    tree = ref["layers"]

    def refusal(name, input_, code, message, path, doc=ref):
        return {"name": name, "setup": doc, "steps": [
            step("execute", input_, failed(code, message, path),
                 {"canUndo": False, "dirty": False, "revision": "same", "layers": doc["layers"]})]}
    cases.append(refusal("bilinmeyen profil", {"name": "yok"}, "unknown_profile", UNKNOWN, "name"))
    cases.append(refusal("profil verilmedi", {}, "unknown_profile", NONE_GIVEN, "name"))
    for title, p, why in INVALID:
        cases.append(refusal(f"geçersiz profil: {title}", {"profile": p}, "invalid_profile", f"Veri profili geçersiz: {why}", "profile"))
    cases.append(refusal("geçersiz beklenen sürüm", {**NAME, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION,
                         "expectedRevision"))
    first = step("execute", {"uids": ["$uidOf:4"], "operation": "attributes", "attrs": {"Nitelik": "tarla"}},
                 completed({"changed": ["$uidOf:4"], "ids": [4], "revision": "$current"}), command="cad.entities.set")
    cases.append({"name": "sürüm çakışması: hiçbir şey yazılmaz", "setup": ref, "steps": [
        step("captureRevision", **{"as": "once"}),
        first,
        step("execute", {**NAME, "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision",
                                              "revision": "$current"}},
             {"revision": "same", "layers": tree}),
    ]})
    del tree
    return cases


def check_case(name, doc, input_=NAME, links=LINKS, plan=False, profile=PROFILE, before=()):
    rep = report(linked(doc, list(links)), profile)
    steps = list(before) + link_steps(list(links))
    if plan:
        steps.append(step("plan", input_, completed(rep), {"revision": "same"}))
    steps.append(step("execute", input_, completed(rep), {"revision": "same", "layers": doc["layers"]}))
    return {"name": name, "setup": doc, "steps": steps}


def clean_doc():
    """The reference with its data put right and the profile applied: no finding."""
    d = reference()
    out, _ = apply(d, PROFILE)
    fresh = {}
    k = 0
    for i in out["layers"]:
        k += 1
        fresh[i] = f"profil-{k}"

    def rename(nodes):
        for n in nodes:
            if n["id"] in fresh:
                n["id"] = fresh[n["id"]]
            rename(n["children"])
    rename(d["layers"])
    for r in d["settings"].get("relations", []):
        r["parent"], r["child"] = fresh.get(r["parent"], r["parent"]), fresh.get(r["child"], r["child"])
    ents = []
    for e in d["entities"]:
        a = e["attrs"]
        if e["id"] == 6:
            a["TapuAlani"] = "800.00"
        if e["id"] == 7:
            a["Nitelik"] = "arsa"
        if e["id"] == 9:
            a.update({"ParselKimlik": "102/2", "ParselNo": "2"})
        if e["id"] == 11:
            a["KatSayisi"] = "5"
        if e["id"] == 12:
            a.pop("Parsel")
        if e["id"] == 14:
            continue
        if e["id"] == 16:
            a["Tur"] = "sokak"
        if e["id"] == 19:
            a["KapiNo"] = "3"
        if e["id"] == 20:
            a["PostaKodu"] = "58040"
        ents.append(e)
    d["entities"] = ents
    return d


def check_cases():
    ref = reference()
    cases = [check_case("Belediye örnek projesi: bütün bulgular sırasıyla; plan aynı cevabı verir", ref, plan=True)]
    # After apply: what Uygula fixes is gone; the data's own findings stay.
    applied_doc = linked(ref)
    out, w = apply(applied_doc, PROFILE)
    after_apply = report(applied_doc, PROFILE)
    cases.append({"name": "Uygula'dan sonra yalnız verinin kendi bulguları kalır", "setup": ref,
                  "steps": link_steps() + [step("execute", NAME, completed(out, w), command=APPLY),
                                           step("execute", NAME, completed(after_apply), {"revision": "same"})]})
    cases.append(check_case("Verisi düzeltilmiş ve profili uygulanmış proje: bulgu yok, clean", clean_doc(),
                            links=[(10, "Parsel", 4), (11, "Parsel", 5), (13, "Parsel", 6), (17, "Bina", 10), (18, "Bina", 11)]))
    # The hole is taken off: a registered area that is the gross area differs by 100 m².
    ents = reference_entities()
    next(e for e in ents if e["id"] == 8)["attrs"]["TapuAlani"] = "1600"
    cases.append(check_case("Delikli parselin geometrik alanı delik çıkarılarak: brüt kayıtlı alan 100 m² farklı", reference(entities=ents)))
    # The tolerance: a difference equal to it passes, one just past it does not.
    ents = reference_entities()
    next(e for e in ents if e["id"] == 5)["attrs"]["TapuAlani"] = "799.00"
    next(e for e in ents if e["id"] == 4)["attrs"]["TapuAlani"] = "801.25"
    cases.append(check_case("Tolerans: tam 1 m² fark geçer, 1,25 m² geçmez", reference(entities=ents)))
    # A field found by its key, not its name.
    lay = reference_layers()
    for f in get(lay, "parsel")["fields"]:
        if f["name"] == "TapuAlani":
            f["id"] = "TapuAlani"
            f["name"] = "Tapu alanı"
    cases.append(check_case("Alan adıyla değil anahtarıyla bulunur (Tapu alanı, anahtarı TapuAlani)", reference(layers=lay)))
    # The field of another kind.
    lay = reference_layers()
    for f in get(lay, "bina")["fields"]:
        if f["name"] == "KatSayisi":
            f.clear()
            f.update({"name": "KatSayisi", "alias": "Kat sayısı", "kind": "text", "length": 10})
    cases.append(check_case("Türü farklı alan söylenir; değerleri profilin kuralıyla denetlenir", reference(layers=lay)))
    # Upper-case names are found by the Turkish fold.
    lay = reference_layers()
    get(lay, "kadastro")["name"] = "KADASTRO"
    get(lay, "parsel")["name"] = "PARSEL"
    get(lay, "idari")["name"] = "İDARİ BİRİMLER"
    cases.append(check_case("Büyük harfli adlar Türkçe katlanarak bulunur (İDARİ BİRİMLER, KADASTRO › PARSEL)", reference(layers=lay)))
    # A group where the class's layer should be.
    lay = reference_layers()
    get(lay, "ulasim")["children"] = [node("yol-grup", "Yol orta hattı", "group", [node("yol", "Eski yollar",
                                                                                          style={**NEW_STYLE, "color": "#F5A524"},
                                                                                          fields=get(reference_layers(), "yol")["fields"])])]
    cases.append(check_case("Sınıfın katmanının yerinde grup: eksik katman, Uygula düzeltmez", reference(layers=lay)))
    # Three objects share an identifier.
    ents = reference_entities()
    next(e for e in ents if e["id"] == 5)["attrs"]["ParselKimlik"] = "101/1"
    cases.append(check_case("Üç nesnede aynı kimlik", reference(entities=ents)))
    # A whitespace-only required value is empty.
    ents = reference_entities()
    next(e for e in ents if e["id"] == 2)["attrs"]["AdaNo"] = "   "
    cases.append(check_case("Yalnız boşluk olan zorunlu değer boştur", reference(entities=ents)))
    # The profile itself.
    small_doc = reference(layers=[node("peyzaj", "Peyzaj", "group", [
        node("agac", "Ağaç", style={**NEW_STYLE, "color": "#30A46C"}, fields=follow(SMALL["classes"][0]["fields"], SMALL["domains"])),
        node("yesil", "Yeşil alan", style={**NEW_STYLE, "color": "#46A758"}, fields=SMALL["classes"][1]["fields"])])],
        entities=[point(1, "agac", 5, 5, {"AgacNo": "A-1", "Tur": "cinar"}),
                  point(2, "agac", 9, 5, {"AgacNo": "A-1", "Tur": "palmiye"}),
                  polygon(3, "agac", rect(0, 0, 2, 2), {"AgacNo": "A-3"}),
                  polygon(4, "yesil", rect(0, 20, 30, 10), {"Ad": "Park", "KayitliAlan": "300.4"}),
                  polygon(5, "yesil", rect(40, 20, 10, 10), {"Ad": "Bahçe", "KayitliAlan": "101"})],
        domains=SMALL["domains"], relations=None)
    cases.append(check_case("Profil kendisiyle: başka geometri, liste dışı değer, yinelenen kimlik, kimliksiz sınıfın alan farkı",
                            small_doc, {"profile": SMALL}, links=[], profile=SMALL))
    # A centimetre drawing: the area in square metres.
    cm = copy.deepcopy(small_doc)
    cm["settings"]["drawingUnit"] = "cm"
    for e in cm["entities"]:
        for p in e.get("pts", []):
            p["x"], p["y"] = X + (p["x"] - X) * 100, Y + (p["y"] - Y) * 100
        if "p" in e:
            e["p"] = {"x": X + (e["p"]["x"] - X) * 100, "y": Y + (e["p"]["y"] - Y) * 100}
    cases.append(check_case("Santimetre çiziminde alan metrekareye çevrilerek karşılaştırılır", cm, {"profile": SMALL}, links=[],
                            profile=SMALL))
    # Only the address data missing.
    d = clean_doc()
    loc_have = d["settings"].pop("addressLocators")
    del loc_have
    cases.append(check_case("Yalnız adres verisi eksik", d,
                            links=[(10, "Parsel", 4), (11, "Parsel", 5), (13, "Parsel", 6), (17, "Bina", 10), (18, "Bina", 11)]))
    # A relation's link to an object not of the parent layer.
    cases.append(check_case("Bağ üst katmanda olmayan nesneyi anıyor (adres → parsel)", ref,
                            links=LINKS[:5] + [(19, "Bina", 4)]))
    # Validate and plan.
    cases.append({"name": "Doğrulama: profil adıyla", "setup": ref, "steps": [step("validate", NAME, VALID)]})
    cases.append({"name": "bilinmeyen profil", "setup": ref, "steps": [step("execute", {"name": "yok"}, failed("unknown_profile", UNKNOWN, "name"))]})
    cases.append({"name": "profil verilmedi", "setup": ref, "steps": [step("execute", {}, failed("unknown_profile", NONE_GIVEN, "name"))]})
    for title, p, why in INVALID[:4]:
        cases.append({"name": f"geçersiz profil: {title}", "setup": ref,
                      "steps": [step("execute", {"profile": p}, failed("invalid_profile", f"Veri profili geçersiz: {why}", "profile"))]})
    return cases


def files():
    def wrap(command, title, cases):
        return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
                "note": "Written by scripts/fixtures/profile_cases.py from the contract's rules and docs/adr/0541; do not edit by hand.",
                "setup": reference(), "cases": cases}
    return {
        OUT / f"{APPLY}.json": wrap(APPLY, "Veri profilini uygula", apply_cases()),
        OUT / f"{CHECK}.json": wrap(CHECK, "Veri profilini denetle", check_cases()),
        REF / "belediye.json": reference(),
        REF / "links.json": {"format": "kentos.profile-links", "version": 1,
                             "note": "The reference project's relation links: the child object's field holds the parent object's persistent id; written by cad.entities.set after opening.",
                             "links": [{"child": c, "field": f, "parent": p} for c, f, p in LINKS]},
    }


def main() -> int:
    bad_files = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad_files += 1
            else:
                print(f"{path.name} güncel" + (f": {len(doc['cases'])} durum." if "cases" in doc else "."))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)}" + (f" ({len(doc['cases'])} durum)" if "cases" in doc else ""))
    return 1 if bad_files else 0


if __name__ == "__main__":
    sys.exit(main())
