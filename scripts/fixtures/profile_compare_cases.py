#!/usr/bin/env python3
"""The shared cases of `cad.profiles.compare`, the catalog's migration report (plan/map-cad.md MAP-04, docs/adr/4940),
written from the ADR without KentOS code.

    python3 scripts/fixtures/profile_compare_cases.py           # writes the cases
    python3 scripts/fixtures/profile_compare_cases.py --check   # compares

The drawing is a map project with the layers of KentOS's `harita-varliklari` catalog (its classes' objects, the field
codes and the value lists' codes in their fields) and five reconcile decisions recorded as resolved review notes. The
new catalog is an institution's `belediye-harita` release 2, written here: the control points split by their codes,
the pole renamed, kerbs and road edges merged, the tree retired, a manhole's field dropped and another retyped, a
door's number renamed, a wall drawn as an area, a sign class added; a code of the network list removed, one retired,
one relabelled, one added; the pole kind list dropped, a sign list added.

The rules (ADR 4940), written here by hand:

- an old class's targets: the new class of its id, else the new classes (in their order) holding one of its codes;
  none: `retired`; two or more: `split`; one by its id: `same` when the folded layer path, the geometry, the codes and
  the fields hold, else `changed`; one by its codes: `merged` when that class carries an old class's id or another
  old class without an id match leads to it, else `renamed`; a new class no old class leads to: `added`, after;
- a class's objects: every object of the layer its old path leads to (the names Turkish-folded), in the drawing's order;
- the fields of a class with one target, in the old class's order: `removed` (not in the new class by name; objects
  holding a value), `kindChanged`, `listChanged`, `nowRequired` (objects without a value), the first that holds; then
  the new class's fields the old has not, `added` (objects without a value); a field is described by its kind's word,
  then its list;
- the codes, per old coded list: a code the new list lacks (or the list is gone) `removed`, one the new list retires
  `retired`, a new label `relabelled`; then the new list's new codes `added`; then the lists only the new catalog has,
  their codes `added`; a code's objects: those of the old classes' layers whose field linked to the list holds it (each
  of a multiple field's `;`-joined codes);
- the decisions: the resolved review notes whose decision is the reconcile command's and not a rejection, by number;
  the class the correction names when the old catalog has it, else the old class whose layer holds the asset, else the
  note is left out; `classRetired`, `classSplit`, else `fieldDropped` when a field it gave is removed, else `follows`
  (renamed or merged) or `kept`;
- `decide`: a retired or split class with objects, a class whose geometry changes with objects; a field row with
  objects, but an added one only when it is required; a removed or retired code with objects; a decision dropped,
  retired or split. The report writes nothing.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.profiles.compare.json"
OLD = json.loads((ROOT / "catalog/v1/profiles/harita-varliklari.json").read_text("utf-8"))
X0, Y0 = 500000.0, 4400000.0
G = "Harita varlıkları"
RECONCILE = "map.features.reconcile"
KIND_WORDS = {"text": "metin", "integer": "tam sayı", "decimal": "ondalık sayı", "date": "tarih",
              "boolean": "evet/hayır"}
GEOMETRY_WORDS = {"area": "alan", "line": "çizgi", "point": "nokta", "record": "kayıt"}
NOTES = ["Rapor çizime bir şey yazmaz: sınıflar kalıcı kimlikleriyle, yoksa saha kodlarıyla izlenir; addan ya da "
         "konumdan tahmin yapılmaz.",
         "İki kataloğun karşılaştırılması mevzuata ya da TUCBS kayıtlarına uygunluk iddiası değildir."]


# ── The new catalog ──────────────────────────────────────────────────────────────────────────────────────────────


def old_class(cid):
    return copy.deepcopy(next(c for c in OLD["classes"] if c["id"] == cid))


def old_domain(did):
    return copy.deepcopy(next(d for d in OLD["domains"] if d["id"] == did))


def new_catalog():
    sebeke = old_domain("sebeke")
    values = []
    for v in sebeke["values"]:
        if v["code"] == "dogalgaz":
            continue                                   # removed
        if v["code"] == "telekom":
            v["retired"] = True                        # retired
        if v["code"] == "yagmursuyu":
            v["label"] = "Yağmursuyu"                  # relabelled
        values.append(v)
    values.append({"code": "atiksu", "label": "Atık su"})
    sebeke["values"] = values
    levha = {"id": "levha-tur", "name": "Levhanın türü", "kind": "text",
             "values": [{"code": "trafik", "label": "Trafik levhası"}, {"code": "yon", "label": "Yön levhası"}]}

    kn = old_class("kontrol-noktasi")
    gnss = dict(kn, id="kontrol-noktasi-gnss", title="GNSS noktası", layer=[G, "GNSS noktaları"], codes=["KN"])
    poligon = dict(kn, id="poligon-noktasi", title="Poligon noktası", layer=[G, "Poligon noktaları"], codes=["PN"])
    rogar = old_class("rogar")
    rogar["fields"] = [f for f in rogar["fields"] if f["name"] != "KapakKotu"]
    for f in rogar["fields"]:
        if f["name"] == "Derinlik":
            f.clear()
            f.update({"name": "Derinlik", "kind": "text", "alias": "Derinlik", "length": 20})
    rogar["fields"].append({"name": "Malzeme", "kind": "text", "length": 30, "required": True})
    vana = old_class("vana")
    vana["layer"] = ["Altyapı", "Vanalar"]
    direk = old_class("direk")
    direk.update(id="elektrik-diregi", title="Elektrik direği", layer=[G, "Elektrik direkleri"])
    for f in direk["fields"]:
        if f["name"] == "Tur":
            del f["domain"]
            f["length"] = 30
    kapi = old_class("kapi")
    kapi["fields"] = [{"name": "NumaraTaj", "kind": "text", "alias": "Numarataj", "length": 20, "required": True}]
    sinir = old_class("bordur")
    sinir.update(id="yol-siniri", title="Yol sınırı", layer=[G, "Yol sınırları"], codes=["BRD", "YK"])
    duvar = old_class("duvar")
    duvar["geometry"] = "area"
    bina = old_class("bina-izi")
    for f in bina["fields"]:
        if f["name"] == "KatSayisi":
            f["required"] = True
    levhalar = {"id": "isaret-levhasi", "title": "İşaret levhası", "theme": "Ulaşım", "layer": [G, "Levhalar"],
                "color": "#E5484D", "geometry": "point",
                "fields": [{"name": "Tur", "kind": "text", "alias": "Tür", "domain": "levha-tur"}], "codes": ["LVH"]}
    return {"format": "kentos.data-profile", "version": 1, "name": "belediye-harita",
            "title": "Belediye harita kataloğu", "description": "Kurumun büyük ölçekli harita nesne kataloğu.",
            "basis": "Kurumun çalışma kataloğu; deneme verisi.", "release": 2,
            "domains": [old_domain("kontrol-tur"), sebeke, levha],
            "classes": [gnss, poligon, rogar, vana, direk, kapi, sinir, duvar, bina, old_class("yesil-alan"),
                        levhalar]}


NEW = new_catalog()


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────


def uid(n):
    return f"0192f5c0-4940-7000-8000-{n:012x}"


def layer(id_, name, kind="layer", children=()):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": list(children)}


def pt(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def point(lid, attrs, x, y):
    return {"kind": "point", "layerId": lid, "attrs": attrs, "p": pt(x, y)}


def line(lid, attrs, pts):
    return {"kind": "polyline", "layerId": lid, "attrs": attrs, "pts": [pt(x, y) for x, y in pts]}


def area(lid, attrs, x0, y0, x1, y1):
    return {"kind": "polygon", "layerId": lid, "attrs": attrs,
            "pts": [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]}


LAYERS = {"kontrol-noktasi": "hv-kn", "rogar": "hv-rog", "vana": "hv-van", "direk": "hv-drk", "agac": "hv-agc",
          "kapi": "hv-kap", "bordur": "hv-brd", "yol-kenari": "hv-yk", "duvar": "hv-dvr", "bina-izi": "hv-bna"}
CAND = "aday-orto"
OBJECTS = [
    point("hv-kn", {"Kod": "KN", "Ad": "K1", "Tur": "gnss"}, 0, 0),                       # 1
    point("hv-kn", {"Kod": "PN", "Ad": "P7", "Tur": "poligon"}, 10, 0),                   # 2
    point("hv-rog", {"Kod": "ROG", "Ad": "R1", "Sebeke": "kanalizasyon", "KapakKotu": "851.20",
                     "Derinlik": "2.10"}, 20, 0),                                         # 3
    point("hv-rog", {"Kod": "ROG", "Ad": "R2", "Sebeke": "yagmursuyu", "Derinlik": "1.80"}, 30, 0),  # 4
    point("hv-van", {"Kod": "VAN", "Sebeke": "dogalgaz"}, 40, 0),                          # 5
    point("hv-drk", {"Kod": "DRK", "Ad": "D1", "Tur": "aydinlatma"}, 50, 0),               # 6
    point("hv-drk", {"Kod": "DRK", "Ad": "D2", "Tur": "elektrik"}, 60, 0),                 # 7
    point("hv-agc", {"Kod": "AGC", "Cins": "Çınar"}, 70, 0),                               # 8
    point("hv-kap", {"Kod": "KAP", "KapiNo": "14"}, 80, 0),                                # 9
    point("hv-kap", {"Kod": "KAP", "KapiNo": "  "}, 90, 0),                                # 10 a blank number
    line("hv-brd", {"Kod": "BRD1", "Ad": "B1"}, [(0, 10), (20, 10)]),                      # 11
    line("hv-yk", {"Kod": "YK1"}, [(0, 12), (20, 12)]),                                    # 12
    line("hv-dvr", {"Kod": "DVR1", "Ad": "W1"}, [(0, 20), (10, 20)]),                      # 13
    area("hv-bna", {"Kod": "BNA", "Ad": "Bina A", "KatSayisi": "3"}, 0, 30, 10, 40),       # 14
    area("hv-bna", {"Kod": "BNA", "Ad": "Bina B"}, 20, 30, 30, 40),                        # 15
    point(CAND, {"Kod": "ROG", "Güven": "0.8"}, 20.1, 0),                                  # 16 candidates
    point(CAND, {"Kod": "AGC", "Güven": "0.7"}, 50.1, 0),                                  # 17
    point(CAND, {"Kod": "KAP", "Güven": "0.9"}, 80.1, 0),                                  # 18
    point(CAND, {"Kod": "AGC", "Güven": "0.6"}, 70.1, 0),                                  # 19
    point(CAND, {"Kod": "KN", "Güven": "0.9"}, 0.1, 0),                                    # 20
    point(CAND, {"Kod": "XX", "Güven": "0.4"}, 99, 0),                                     # 21
    point(CAND, {"Kod": "XX", "Güven": "0.4"}, 98, 0),                                     # 22
]


def note(number, cand, asset, **decision):
    anchors = [{"uid": uid(cand), "stamp": "s"}] + ([{"uid": uid(asset), "stamp": "s"}] if asset else [])
    d = {"command": RECONCILE, "candidate": uid(cand), "source": "Ortofoto"}
    d.update(decision)
    if asset:
        d["asset"] = uid(asset)
    return {"id": f"0192f5b0-4940-7000-8000-{number:012x}", "number": number, "text": "Detay eşleştirme kararı.",
            "status": "resolved", "author": "Ayşe Demir", "anchors": anchors, "decision": d}


def review_notes():
    return [
        note(1, 16, 3, accept=True),                                  # a manhole accepted: kept
        note(2, 17, 6, **{"class": "direk"}),                         # a tree corrected to a pole: follows
        note(3, 18, 9, fields={"KapiNo": "14"}),                      # a door's number given: dropped
        note(4, 19, 8, accept=True),                                  # a tree accepted: its class retired
        note(5, 20, 1, accept=True),                                  # a control point: its class split
        note(6, 21, None, reject="Gölge; nesne yok"),                 # a rejection: left out
        note(7, 22, 21, **{"class": "yok-boyle"}),                    # no class of the catalog, its asset not on a class layer: left out
        {"id": "0192f5b0-4940-7000-8000-000000000008", "number": 8, "text": "Paftayı kontrol edin.",
         "status": "open"},                                           # no decision: left out
    ]


def setup(objects=OBJECTS, notes=True, layers=True):
    names = {c["id"]: c["layer"][-1] for c in OLD["classes"]}
    kids = [layer(lid, names[cid]) for cid, lid in LAYERS.items()] if layers else []
    lays = [layer("0", "0"), layer("hv", G, "group", kids), layer(CAND, "Adaylar (ortofoto)"), layer("cizim", "Çizim")]
    ents = []
    for i, e in enumerate(objects, 1):
        x = copy.deepcopy(e)
        x["id"] = i
        if not layers and x["layerId"].startswith("hv-"):
            x["layerId"] = "cizim"
        ents.append(x)
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"}
    if notes:
        settings["reviewNotes"] = review_notes()
    return {"format": "kentos.document", "version": 2, "name": "Katalog geçişi", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": lays, "activeLayer": "cizim", "entities": ents,
            "uids": [uid(i) for i in range(1, len(ents) + 1)], "styles": {"items": [], "categories": []}}


# ── The rules ────────────────────────────────────────────────────────────────────────────────────────────────────


def fold(text):
    """Turkish case folding, a character at a time (I → ı, İ → i)."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower() if len(c.lower()) == 1 else c) for c in text)


def find_layer(d, path):
    """The layer the path leads to: each name folded, groups then the last a layer; none otherwise."""
    here = d["layers"]
    for i, name in enumerate(path):
        n = next((n for n in here if fold(n["name"].strip()) == fold(name.strip())), None)
        if n is None:
            return None
        last = i + 1 == len(path)
        if last:
            return n["id"] if n["type"] == "layer" else None
        if n["type"] != "group":
            return None
        here = n.get("children", [])
    return None


def objects_of(d, c):
    lid = find_layer(d, c["layer"])
    if lid is None:
        return []
    return [(d["uids"][i], e) for i, e in enumerate(d["entities"]) if e["layerId"] == lid]


def value(e, name):
    v = e.get("attrs", {}).get(name)
    v = v.strip() if isinstance(v, str) else None
    return v or None


def described(f):
    w = KIND_WORDS[f["kind"]]
    return f"{w}, {f['domain']}" if f.get("domain") else w


def words(path):
    return " › ".join(path)


def field_rows(o, t, objs):
    holding = lambda n: [u for u, e in objs if value(e, n) is not None]
    lacking = lambda n: [u for u, e in objs if value(e, n) is None]
    rows = []

    def row(name, kind, was, becomes, touched, decide, note):
        rows.append({"class": o["id"], "toClass": t["id"], "field": name, "kind": kind, "was": was,
                     "becomes": becomes, "objects": len(touched), "uids": touched[:1000], "decide": decide,
                     "note": note})

    new_by = {g["name"]: g for g in t["fields"]}
    for f in o["fields"]:
        g = new_by.get(f["name"])
        if g is None:
            h = holding(f["name"])
            row(f["name"], "removed", described(f), "", h, bool(h),
                f"Yeni sınıfta yok; {len(h)} nesnenin değeri taşınamaz." if h else "Yeni sınıfta yok.")
            continue
        if g["kind"] != f["kind"]:
            kind, touched, head, tail = "kindChanged", holding(f["name"]), "Tür değişir", "değer yeniden okunmalı"
        elif g.get("domain") != f.get("domain"):
            kind, touched, head, tail = "listChanged", holding(f["name"]), "Değer listesi değişir", "değer yeniden okunmalı"
        elif g.get("required", False) and not f.get("required", False):
            kind, touched, head, tail = "nowRequired", lacking(f["name"]), "Zorunlu olur", "nesnede boş"
        else:
            continue
        row(f["name"], kind, described(f), described(g), touched, bool(touched),
            f"{head}; {len(touched)} {tail}." if touched else f"{head}.")
    old_names = {f["name"] for f in o["fields"]}
    for g in t["fields"]:
        if g["name"] in old_names:
            continue
        lack = lacking(g["name"])
        req = g.get("required", False)
        note = ("Yeni zorunlu alan; %d nesnede boş." % len(lack) if lack else "Yeni zorunlu alan.") if req else "Yeni alan."
        row(g["name"], "added", "", described(g), lack, req and bool(lack), note)
    return rows


def parts(o, t, n_fields):
    out = []
    if [fold(x.strip()) for x in o["layer"]] != [fold(x.strip()) for x in t["layer"]]:
        out.append(f"katman {words(o['layer'])} → {words(t['layer'])}")
    if o["geometry"] != t["geometry"]:
        out.append(f"geometri {GEOMETRY_WORDS[o['geometry']]} → {GEOMETRY_WORDS[t['geometry']]}")
    oc, tc = o.get("codes", []), t.get("codes", [])
    if oc != tc:
        out.append(f"kodlar {', '.join(oc) or 'yok'} → {', '.join(tc) or 'yok'}")
    if n_fields:
        out.append(f"{n_fields} alan değişikliği")
    return out


def code_rows(d, old, new):
    holders = {}
    order = {u: i for i, u in enumerate(d["uids"])}
    for c in old["classes"]:
        linked = [f for f in c["fields"] if f.get("domain")]
        for u, e in objects_of(d, c) if linked else []:
            for f in linked:
                v = value(e, f["name"])
                if v is None:
                    continue
                codes = [s.strip() for s in v.split(";") if s.strip()] if f.get("multiple") else [v]
                for code in codes:
                    holders.setdefault((f["domain"], code), set()).add(u)
    held = lambda dom, code: sorted(holders.get((dom, code), set()), key=order.get)
    rows = []

    def row(dom, code, kind, label, to_label, touched, decide, note):
        rows.append({"domain": dom, "code": code, "kind": kind, "label": label, "toLabel": to_label,
                     "objects": len(touched), "uids": touched[:1000], "decide": decide, "note": note})

    new_domains = {x["id"]: x for x in new.get("domains", [])}
    old_ids = {x["id"] for x in old.get("domains", [])}
    for dm in old.get("domains", []):
        if "values" not in dm:
            continue
        nd = new_domains.get(dm["id"])
        nvals = nd.get("values") if nd else None
        for v in dm["values"]:
            w = next((w for w in nvals if w["code"] == v["code"]), None) if nvals is not None else None
            if w is None:
                if v.get("retired"):
                    continue
                h = held(dm["id"], v["code"])
                head = "Yeni listede yok" if nvals is not None else "Liste yeni katalogda yok"
                row(dm["id"], v["code"], "removed", v["label"], "", h, bool(h),
                    f"{head}; {len(h)} nesnede kullanılıyor." if h else f"{head}.")
            elif w.get("retired") and not v.get("retired"):
                h = held(dm["id"], v["code"])
                row(dm["id"], v["code"], "retired", v["label"], w["label"], h, bool(h),
                    f"Yeni listede kullanımdan kalkmış; {len(h)} nesnede kullanılıyor." if h
                    else "Yeni listede kullanımdan kalkmış.")
            elif w["label"] != v["label"]:
                row(dm["id"], v["code"], "relabelled", v["label"], w["label"], held(dm["id"], v["code"]), False,
                    f"Etiket “{v['label']}” → “{w['label']}”.")
        if nvals is not None:
            olds = {v["code"] for v in dm["values"]}
            for w in nvals:
                if not w.get("retired") and w["code"] not in olds:
                    row(dm["id"], w["code"], "added", w["label"], w["label"], [], False, "Yeni kod.")
    for nd in new.get("domains", []):
        if nd["id"] in old_ids or "values" not in nd:
            continue
        for w in nd["values"]:
            if not w.get("retired"):
                row(nd["id"], w["code"], "added", w["label"], w["label"], [], False, "Yeni kod.")
    return rows


def compare(d, old, new):
    oc, nc = old["classes"], new["classes"]
    new_ids = [c["id"] for c in nc]
    old_ids = {c["id"] for c in oc}
    targets = []
    for c in oc:
        if c["id"] in new_ids:
            targets.append([new_ids.index(c["id"])])
        else:
            targets.append([k for k, n in enumerate(nc) if set(n.get("codes", [])) & set(c.get("codes", []))])
    by_id = [c["id"] in new_ids for c in oc]
    classes, fields, info, layer_class = [], [], {}, {}
    for i, c in enumerate(oc):
        lid = find_layer(d, c["layer"])
        if lid is not None:
            layer_class.setdefault(lid, c["id"])
        objs = objects_of(d, c)
        ids = [u for u, _ in objs]
        t = targets[i]
        rows = []
        if not t:
            kind, to_layer, decide, note = "retired", "", bool(ids), "Yeni katalogda yok."
        elif len(t) == 1:
            n = nc[t[0]]
            rows = field_rows(c, n, objs)
            p = parts(c, n, len(rows))
            if by_id[i]:
                kind = "same" if not p else "changed"
            else:
                shared = n["id"] in old_ids or any(j != i and not by_id[j] and t[0] in targets[j]
                                                   for j in range(len(oc)))
                kind = "merged" if shared else "renamed"
            tail = (": " + "; ".join(p)) if p else ""
            note = {"same": "Aynı.", "changed": f"Değişti: {'; '.join(p)}.",
                    "renamed": f"“{n['id']}” olarak sürer{tail}.",
                    "merged": f"“{n['id']}” sınıfında birleşir{tail}."}[kind]
            to_layer = words(n["layer"])
            decide = bool(ids) and c["geometry"] != n["geometry"]
        else:
            moves = []
            for code in c.get("codes", []):
                went = next((nc[j]["id"] for j in t if code in nc[j].get("codes", [])), "yok")
                moves.append(f"{code} → {went}")
            kind, to_layer, decide, note = "split", "", bool(ids), f"Kodları bölünür: {', '.join(moves)}."
        to = [nc[k]["id"] for k in t]
        info[c["id"]] = (kind, to, [r["field"] for r in rows if r["kind"] == "removed"])
        fields.extend(rows)
        classes.append({"from": c["id"], "to": to, "kind": kind, "title": c["title"], "layer": words(c["layer"]),
                        "toLayer": to_layer, "codes": c.get("codes", []),
                        "toCodes": [k for j in t for k in nc[j].get("codes", [])], "objects": len(ids),
                        "uids": ids[:1000], "decide": decide, "note": note})
    reached = {k for t in targets for k in t}
    for k, n in enumerate(nc):
        if k in reached:
            continue
        classes.append({"to": [n["id"]], "kind": "added", "title": n["title"], "layer": words(n["layer"]),
                        "toLayer": words(n["layer"]), "codes": n.get("codes", []), "toCodes": n.get("codes", []),
                        "objects": 0, "uids": [], "decide": False, "note": "Yeni sınıf."})
    codes = code_rows(d, old, new)
    where = {u: e for u, e in zip(d["uids"], d["entities"])}
    decisions = []
    notes = [n for n in d["settings"].get("reviewNotes", []) if n.get("status") == "resolved" and n.get("decision")]
    for n in sorted(notes, key=lambda n: n["number"]):
        dc = n["decision"]
        if dc["command"] != RECONCILE or dc.get("reject") is not None:
            continue
        cls = dc.get("class") if dc.get("class") in old_ids else None
        if cls is None and dc.get("asset") in where:
            cls = layer_class.get(where[dc["asset"]]["layerId"])
        if cls is None:
            continue
        ck, to, removed = info[cls]
        dropped = [k for k in sorted(dc.get("fields", {})) if k in removed]
        if ck == "retired":
            kind = "classRetired"
        elif ck == "split":
            kind = "classSplit"
        elif dropped:
            kind = "fieldDropped"
        elif ck in ("renamed", "merged"):
            kind = "follows"
        else:
            kind = "kept"
        note = {"kept": "Karar korunur.", "follows": f"Karar “{', '.join(to)}” sınıfıyla sürer.",
                "fieldDropped": f"Verilen alan kalkar: {', '.join(dropped)}.",
                "classRetired": "Kararın sınıfı yeni katalogda yok.",
                "classSplit": "Kararın sınıfı bölünür: yeni sınıfı seçilmeli."}[kind]
        row = {"number": n["number"], "candidate": dc["candidate"]}
        if dc.get("asset"):
            row["asset"] = dc["asset"]
        row.update({"class": cls, "to": to, "kind": kind, "fields": dropped,
                    "decide": kind not in ("kept", "follows"), "note": note})
        decisions.append(row)
    decide = sum(r["decide"] for rows in (classes, fields, codes, decisions) for r in rows)
    side = lambda p: {"name": p["name"], "title": p["title"], "release": p.get("release", 1)}
    return {"from": side(old), "to": side(new), "classes": classes, "fields": fields, "codes": codes,
            "decisions": decisions, "decide": decide, "notes": NOTES, "revision": "$current"}


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────


SAME = {"revision": "same"}


def completed(out):
    return {"status": "completed", "output": out, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def steps(d, inp, old, new):
    out = compare(d, old, new)
    return [{"op": "validate", "input": inp, "result": completed(None), "expect": SAME},
            {"op": "plan", "input": inp, "result": completed(out), "expect": SAME},
            {"op": "execute", "input": inp, "result": completed(out), "expect": dict(SAME, canUndo=False)}]


def case(name, d, st, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = d
    c["steps"] = st
    return c


def bad_format(p):
    p = copy.deepcopy(p)
    p["format"] = "kentos.veri-profili"
    return p


def cases():
    d = setup()
    to_new = {"from": "harita-varliklari", "toProfile": NEW}
    out = [case("Harita varlıkları → belediye kataloğu: bölünen, yeniden adlandırılan, birleşen, kalkan, değişen ve yeni "
                "sınıflar; alanlar, kodlar ve kayıtlı kararlar", d, steps(d, to_new, OLD, NEW),
                "Rapor bir şey yazmaz: plan ve çalıştırma aynı cevabı verir, çizimin sürümü aynı kalır.")]
    back = {"fromProfile": NEW, "to": "harita-varliklari"}
    out.append(case("Ters yön: belediye kataloğundan harita varlıklarına (katmanları çizimde olmayan sınıflar)",
                    d, steps(d, back, NEW, OLD)))
    same = {"from": "harita-varliklari", "to": "harita-varliklari"}
    out.append(case("Aynı katalog: bütün sınıflar aynı, alan ve kod değişikliği yok, kararlar korunur", d,
                    steps(d, same, OLD, OLD)))
    bare = setup(objects=OBJECTS[15:], notes=False, layers=False)
    out.append(case("Sınıf katmanları olmayan çizim: nesne sayıları sıfır, yalnız katalog farkı; karar gerekmez", bare,
                    steps(bare, to_new, OLD, NEW)))
    multi = copy.deepcopy(NEW)
    rog = next(c for c in multi["classes"] if c["id"] == "rogar")
    old_multi = copy.deepcopy(OLD)
    for c in (rog, next(c for c in old_multi["classes"] if c["id"] == "rogar")):
        for f in c["fields"]:
            if f["name"] == "Sebeke":
                f["multiple"] = True
    md = setup()
    md["entities"][3]["attrs"]["Sebeke"] = "yagmursuyu; dogalgaz"
    out.append(case("Çok değerli alan: her kod ayrı sayılır (yağmur suyu ve doğal gaz aynı rögarda)", md,
                    steps(md, {"fromProfile": old_multi, "toProfile": multi}, old_multi, multi)))
    locked = setup()
    for g in locked["layers"]:
        for k in g.get("children", []):
            if k["id"] in ("hv-drk", "hv-agc"):
                k["locked"], k["visible"] = True, k["id"] != "hv-agc"
    out.append(case("Kilitli ve gizli sınıf katmanının nesneleri de sayılır: rapor yazmaz", locked,
                    steps(locked, to_new, OLD, NEW)))
    upper = setup()
    upper["layers"][1]["name"] = "HARİTA VARLIKLARI"
    upper["layers"][1]["children"][3]["name"] = "DİREKLER"
    out.append(case("Katman yolu Türkçe büyük harfle yazılmış: katlanmış adla bulunur", upper,
                    steps(upper, to_new, OLD, NEW)))
    blocked = setup()
    rog = blocked["layers"][1]["children"][1]
    rog["type"] = "group"
    rog["children"] = [layer("hv-rog-ic", "İç")]
    for e in blocked["entities"]:
        if e["layerId"] == "hv-rog":
            e["layerId"] = "hv-rog-ic"
    out.append(case("Sınıfın yolu bir gruba çıkıyor: katman yok, nesneleri sayılmaz", blocked,
                    steps(blocked, to_new, OLD, NEW)))
    asset_only = setup()
    asset_only["settings"]["reviewNotes"][1]["decision"]["class"] = "agac-degil"
    out.append(case("Kararın düzelttiği sınıf eski katalogda yok: sınıfı varlığın katmanından okunur", asset_only,
                    steps(asset_only, to_new, OLD, NEW)))
    no_list = copy.deepcopy(NEW)
    no_list["domains"] = [x for x in no_list["domains"] if x["id"] != "sebeke"]
    for c in no_list["classes"]:
        for f in c["fields"]:
            if f.get("domain") == "sebeke":
                del f["domain"]
    out.append(case("Şebeke listesi yeni katalogda yok: bütün kodları kalkar, alanların listesi değişir", d,
                    steps(d, {"from": "harita-varliklari", "toProfile": no_list}, OLD, no_list)))
    kept_id = copy.deepcopy(NEW)
    kept_id["classes"][0] = dict(kept_id["classes"][0], id="kontrol-noktasi")
    out.append(case("Kimliği kalan sınıfın kodu başka sınıfa gidiyor: değişti (kodlar), öbür sınıf yeni", d,
                    steps(d, {"from": "harita-varliklari", "toProfile": kept_id}, OLD, kept_id)))
    into_old = copy.deepcopy(NEW)
    for c in into_old["classes"]:
        if c["id"] == "yol-siniri":
            c.update(id="bordur", title="Bordür")
    out.append(case("Yol kenarı eski bordür sınıfına katılıyor: bordür değişti, yol kenarı birleşti", d,
                    steps(d, {"from": "harita-varliklari", "toProfile": into_old}, OLD, into_old)))
    one_code = copy.deepcopy(NEW)
    one_code["classes"] = [c for c in one_code["classes"] if c["id"] != "poligon-noktasi"]
    out.append(case("Bir kodu düşen sınıf tek sınıfa gidiyor: yeniden adlandırıldı, kodlar KN, PN → KN", d,
                    steps(d, {"from": "harita-varliklari", "toProfile": one_code}, OLD, one_code)))
    empty = setup(objects=OBJECTS[15:], notes=False)
    out.append(case("Sınıf katmanları boş: kalkan sınıf, geometrisi değişen duvar ve yeni zorunlu alan karar istemez",
                    empty, steps(empty, to_new, OLD, NEW)))
    filled = setup()
    for e in filled["entities"]:
        if e["layerId"] == "hv-rog":
            e["attrs"]["Malzeme"] = "Beton"
    out.append(case("Yeni zorunlu alanı nesneler zaten taşıyor: eklenir, karar istemez", filled,
                    steps(filled, to_new, OLD, NEW)))
    retired_old = copy.deepcopy(OLD)
    for v in next(x for x in retired_old["domains"] if x["id"] == "sebeke")["values"]:
        if v["code"] == "dogalgaz":
            v["retired"] = True
    out.append(case("Eski listede zaten kullanımdan kalkmış kod yeni listede yoksa satır olmaz", d,
                    steps(d, {"fromProfile": retired_old, "toProfile": NEW}, retired_old, NEW)))
    # Refusals: the old catalog first, then the new one.
    out.append(case("Eski katalog verilmedi: unknown_profile", d, [
        {"op": "validate", "input": {"to": "harita-varliklari"},
         "result": failed("unknown_profile", "Eski kataloğu adıyla (from) ya da kendisiyle (fromProfile) verin.", "from"),
         "expect": SAME}]))
    out.append(case("Yeni katalog bulunamadı: unknown_profile (to)", d, [
        {"op": "plan", "input": {"from": "harita-varliklari", "to": "belediye-harita"},
         "result": failed("unknown_profile", "Yeni katalog “belediye-harita” bulunamadı: KentOS'un profilleri "
                          "tucbs-temel, harita-varliklari, imar-plani; projenin profilleri Veri profili penceresinde "
                          "listelenir.", "to"), "expect": SAME}]))
    out.append(case("Eski katalog geçersiz: invalid_profile (fromProfile), yeni katalog da geçersizken önce eskisi", d, [
        {"op": "execute", "input": {"fromProfile": bad_format(NEW), "toProfile": bad_format(NEW)},
         "result": failed("invalid_profile", "Eski katalog geçersiz: Profilin biçimi “kentos.data-profile”, sürümü 1 "
                          "olmalı.", "fromProfile"), "expect": SAME}]))
    out.append(case("Yeni katalog geçersiz: invalid_profile (toProfile)", d, [
        {"op": "plan", "input": {"from": "harita-varliklari", "toProfile": bad_format(NEW)},
         "result": failed("invalid_profile", "Yeni katalog geçersiz: Profilin biçimi “kentos.data-profile”, sürümü 1 "
                          "olmalı.", "toProfile"), "expect": SAME}]))
    both = {"from": "yok-boyle", "fromProfile": OLD, "toProfile": NEW}
    out.append(case("Kataloğun kendisi verilince adı okunmaz", d, steps(d, both, OLD, NEW)[1:2]))
    return out


def assert_cases():
    r = compare(setup(), OLD, NEW)
    kinds = {c.get("from") or c["to"][0]: c["kind"] for c in r["classes"]}
    assert kinds == {"kontrol-noktasi": "split", "rogar": "changed", "vana": "changed", "direk": "renamed",
                     "agac": "retired", "kapi": "changed", "bordur": "merged", "yol-kenari": "merged",
                     "duvar": "changed", "bina-izi": "changed", "yesil-alan": "same",
                     "isaret-levhasi": "added"}, kinds
    f = {(x["class"], x["field"]): (x["kind"], x["objects"], x["decide"]) for x in r["fields"]}
    assert f[("rogar", "KapakKotu")] == ("removed", 1, True), f
    assert f[("rogar", "Derinlik")] == ("kindChanged", 2, True), f
    assert f[("rogar", "Malzeme")] == ("added", 2, True), f
    assert f[("direk", "Tur")] == ("listChanged", 2, True), f
    assert f[("kapi", "KapiNo")] == ("removed", 1, True), f
    assert f[("bina-izi", "KatSayisi")] == ("nowRequired", 1, True), f
    c = {(x["domain"], x["code"]): (x["kind"], x["objects"]) for x in r["codes"]}
    assert c[("sebeke", "dogalgaz")] == ("removed", 1) and c[("sebeke", "telekom")] == ("retired", 0), c
    assert c[("sebeke", "yagmursuyu")] == ("relabelled", 1) and c[("direk-tur", "aydinlatma")] == ("removed", 1), c
    dk = [(x["number"], x["kind"]) for x in r["decisions"]]
    assert dk == [(1, "kept"), (2, "follows"), (3, "fieldDropped"), (4, "classRetired"), (5, "classSplit")], dk


def main():
    assert_cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.profiles.compare", "commandVersion": 1,
           "title": "Katalog geçiş raporu: sınıflar, alanlar, kodlar ve kayıtlı kararlar",
           "note": "ADR 4940 (MAP-04). Bağımsız başvuru scripts/fixtures/profile_compare_cases.py: kurallar ADR'den; "
                   "KentOS kodu yok. Eski katalog KentOS'un harita-varliklari profili (veri), yenisi betikte yazılan "
                   "kurum kataloğu.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("cad.profiles.compare ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} eski.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
