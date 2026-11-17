#!/usr/bin/env python3
"""Element types and the elements following them (docs/adr/8106; plan/bim-plan.md BIM-12), from the ADR's rules,
without KentOS code.

Writes fixtures/commands/v1/cad.elementtypes.edit.json; `--check` compares and writes nothing. The rules, as the ADR
gives them:

- a type has an id, a name (unique among its kind's, case aside), a kind (column, beam, slab, roof, railing, stair), a
  version and values: its kind's type fields by the element's own names, each as the element writes it (a field given
  its default is left out: a column's `fill: false`, `shape: "rect"`; a number is a real number);
- an element following a type carries `arch.elementType` {id, version, overrides}; it takes each of its kind's type
  fields from the type (its absence where the type gives none; a column's `type` the type's name), but those in
  `overrides`, which stay its own;
- a type's change: the values given set (null takes a field away), the version up by one; each element following it
  takes it, those on a locked layer left at the older version (skipped), those the change makes invalid refused (all or
  none, or with `skip` left at the older version); an element keeping its own value of a changed field is `kept` (and
  still takes the new version, so it is also `applied`);
- after any edit of an element at its type's version, its exceptions are the fields in which it is not its type's.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

# Each kind's type fields in order: (key, label, required, named).
FIELDS = {
    "column": [("member", "Tür", False, False), ("shape", "Biçim", False, False), ("width", "En", True, False),
               ("depth", "Boy", False, False), ("legs", "Kol kalınlıkları", False, False),
               ("height", "Yükseklik", False, False), ("fill", "Dolgu", False, False),
               ("material", "Malzeme", False, False), ("type", "Kolon türü", False, True)],
    "beam": [("role", "Tür", False, False), ("member", "Kayıt", False, False), ("width", "Genişlik", True, False),
             ("height", "Yükseklik", True, False), ("material", "Malzeme", False, False)],
    "slab": [("role", "Döşeme türü", False, False), ("covering", "Kaplama tipi", False, False),
             ("thickness", "Kalınlık", True, False), ("materialId", "Proje malzemesi", False, False),
             ("material", "Malzeme", False, False), ("edge", "Kenar", False, False), ("layers", "Katmanlar", False, False)],
    "roof": [("thickness", "Kalınlık", False, False), ("material", "Kaplama", False, False),
             ("layers", "Katmanlar", False, False), ("gutter", "Oluk", False, False),
             ("snowGuard", "Kar siperliği", False, False)],
    "railing": [("height", "Yükseklik", True, False), ("infill", "Dolgu", False, False),
                ("gap", "Çubuk aralığı", False, False), ("width", "Küpeşte eni", True, False)],
    "stair": [("width", "Genişlik", True, False), ("tread", "Basamak", True, False), ("landing", "Sahanlık", False, False),
              ("winders", "Balansman basamağı", False, False), ("post", "Merkez direği", False, False),
              ("railings", "Korkuluk", False, False), ("numbering", "Basamak numaraları", False, False)],
}
LABEL = {"column": "Kolon", "beam": "Kiriş", "slab": "Döşeme", "roof": "Çatı", "railing": "Korkuluk", "stair": "Merdiven"}
WORDS = {"column": "kolon", "beam": "kiriş", "slab": "döşeme", "roof": "çatı", "railing": "korkuluk", "stair": "merdiven"}
# A field written only when it is not its default (the element leaves the default out).
DEFAULTS = {"shape": "rect", "fill": False, "role": None, "infill": "balusters", "numbering": False}
STEP = "Eleman tipi"
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0}]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def UID(n):
    return f"0192f5a1-7720-7000-8000-{n:012x}"


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def written(key, value):
    """A value as the element writes it: none for a default, a real number for a number."""
    if key in DEFAULTS and value == DEFAULTS[key]:
        return None
    if key == "role" and value in ("floor", "beam"):
        return None
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return float(value)
    return value


def canonical(kind, values):
    out = {}
    for key, _, _, named in FIELDS[kind]:
        if named or key not in values:
            continue
        v = written(key, values[key])
        if v is not None:
            out[key] = v
    return out


def type_json(t):
    out = {"id": t["id"], "name": t["name"], "kind": t["kind"], "version": t["version"]}
    if t["values"]:
        out["values"] = t["values"]
    return out


def id_of(name):
    """A type's id from its name: lower-case ASCII letters and digits, Turkish letters folded, the rest `-`."""
    fold = {"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "I": "i", "İ": "i", "i": "i", "ö": "o", "Ö": "o",
            "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"}
    out = ""
    for c in name:
        if c in fold:
            c = fold[c]
        elif c.isascii() and c.isalnum():
            c = c.lower()
        else:
            c = "-"
        if c == "-" and (not out or out.endswith("-")):
            continue
        out += c
    out = out.rstrip("-") or "tip"
    if out.startswith("kentos-"):
        out = "tip-" + out
    return out[:60]


class Drawing:
    def __init__(self):
        self.types = []
        self.entities = {}
        self.locked = set()

    def settings_arch(self):
        return {"levels": LEVELS, "elementTypes": [type_json(t) for t in self.types]}

    def type(self, id):
        return next(t for t in self.types if t["id"] == id)


def column(id, x, y, layer_id="kolon", **own):
    e = {"kind": "column", "id": id, "layerId": layer_id, "attrs": {}, "at": P(x, y)}
    e.update(own)
    return e


def link(e, t_id, version, overrides=()):
    e.setdefault("arch", {})["elementType"] = {"id": t_id, "version": version}
    if overrides:
        e["arch"]["elementType"]["overrides"] = list(overrides)


def kind_of(e):
    return e["kind"] if e["kind"] in FIELDS else None


def follow(e, t, keep):
    """`e` following `t`, `keep` its own: the type's value of each field not kept (its absence where none)."""
    out = copy.deepcopy(e)
    for key, _, _, named in FIELDS[t["kind"]]:
        if key in keep:
            continue
        if named:
            out[key] = t["name"]
        elif key in t["values"]:
            out[key] = t["values"][key]
        else:
            out.pop(key, None)
    order = [k for k, _, _, _ in FIELDS[t["kind"]] if k in keep]
    out.setdefault("arch", {})["elementType"] = {"id": t["id"], "version": t["version"]}
    if order:
        out["arch"]["elementType"]["overrides"] = order
    return out


def column_problem(e):
    """A column's own rule, as far as the cases reach it: an L's legs thinner than the side they are measured along."""
    if e.get("shape") == "lShape":
        a, b = e["legs"]
        depth = e.get("depth", e["width"])
        for k, (t, side, whose) in enumerate([(a, depth, "boyundan"), (b, e["width"], "eninden")]):
            if t >= side:
                return f"L kolonun kolu {t!r}; kolonun {whose} ({side!r}) ince olmalı."
    return None


def base_drawing():
    d = Drawing()
    d.types = [
        {"id": "s1", "name": "S1", "kind": "column", "version": 2, "values": {"width": 0.3, "depth": 0.5, "fill": True}},
        {"id": "l1", "name": "L1", "kind": "column", "version": 1,
         "values": {"shape": "lShape", "width": 0.6, "depth": 0.6, "legs": [0.25, 0.25]}},
        {"id": "k1", "name": "K1", "kind": "beam", "version": 1, "values": {"width": 0.25, "height": 0.5}},
        {"id": "d15", "name": "D15", "kind": "slab", "version": 1, "values": {"thickness": 0.15, "material": "Betonarme"}},
    ]
    s1, l1, k1 = d.types[0], d.types[1], d.types[2]
    # Seventeen columns of S1 at its version 2 on a 5 m grid; the seventeenth with its own width.
    for k in range(17):
        e = column(1 + k, 5.0 * (k % 6), 5.0 * (k // 6), arch={"level": "zemin"})
        e = follow(e, s1, ["width"] if k == 16 else [])
        if k == 16:
            e["width"] = 0.4
        d.entities[1 + k] = e
    # One on a locked layer and one on an open one, both at S1's version 1, whose depth was 0.6.
    for id, layer_id, x in [(18, "kilitli", 30.0), (19, "kolon", 35.0)]:
        e = column(id, x, 0.0, layer_id=layer_id, arch={"level": "zemin"}, width=0.3, depth=0.6, fill=True, type="S1")
        link(e, "s1", 1)
        d.entities[id] = e
    d.locked.add(18)
    # A column of its own (no type).
    d.entities[20] = column(20, 40.0, 0.0, width=0.45, depth=0.45, material="C30")
    # L1's columns: one following it, one with its own legs.
    d.entities[21] = follow(column(21, 45.0, 0.0), l1, [])
    e = follow(column(22, 50.0, 0.0), l1, ["legs"])
    e["legs"] = [0.35, 0.35]
    d.entities[22] = e
    # A beam of K1, and a slab, a railing, a stair and a roof of their own.
    d.entities[23] = follow({"kind": "beam", "id": 23, "layerId": "kiris", "attrs": {}, "pts": [P(0, 0), P(5, 0)],
                             "arch": {"level": "zemin"}}, k1, [])
    d.entities[24] = {"kind": "slab", "id": 24, "layerId": "doseme", "attrs": {},
                      "parts": [{"outer": {"pts": [P(0, 0), P(10, 0), P(10, 10), P(0, 10)]}}], "thickness": 0.2,
                      "material": "Betonarme"}
    d.entities[25] = {"kind": "railing", "id": 25, "layerId": "korkuluk", "attrs": {}, "pts": [P(0, 20), P(4, 20)],
                      "height": 1.1, "width": 0.05}
    d.entities[26] = {"kind": "stair", "id": 26, "layerId": "merdiven", "attrs": {}, "pts": [P(30, 20), P(33.75, 20)],
                      "width": 1.2, "risers": 16, "riserHeight": 0.175, "tread": 0.25}
    d.entities[27] = {"kind": "roof", "id": 27, "layerId": "cati", "attrs": {},
                      "pts": [P(0, 30), P(6, 30), P(6, 34), P(0, 34)], "slopes": [30.0, None, 30.0, None]}
    d.entities[28] = {"kind": "line", "id": 28, "layerId": "cizim", "attrs": {}, "a": P(0, 40), "b": P(5, 40)}
    return d


BASE = base_drawing()
IDS = sorted(BASE.entities)


def setup():
    ents = [copy.deepcopy(BASE.entities[i]) for i in IDS]
    return {
        "format": "kentos.document", "version": 2, "name": "Eleman tipleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "arch": BASE.settings_arch()},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("kolon", "Kolon"), layer("kilitli", "Kilitli kolonlar", locked=True), layer("kiris", "Kiriş"),
                   layer("doseme", "Döşeme"), layer("korkuluk", "Korkuluk"), layer("merdiven", "Merdiven"),
                   layer("cati", "Çatı"), layer("cizim", "Çizim")],
        "activeLayer": "kolon",
        "styles": {"items": [], "categories": []},
        "entities": ents,
    }


def uid(id):
    return f"$uidOf:{id}"


# ── Results ───────────────────────────────────────────────────────────────────────────────────────────────────────


def edited(d, id, report):
    out = {"types": [type_json(t) for t in d.types], "report": report, "revision": "$current"}
    if id is not None:
        out["id"] = id
    return out


def empty():
    return {"applied": [], "kept": [], "skipped": [], "refused": []}


def warnings_of(report):
    out = []
    if report["kept"]:
        out.append({"code": "element_type_kept",
                    "message": f"{len(report['kept'])} eleman değişen alanda kendi değerini (örnek istisnası) korudu; öbür "
                               f"alanlarda tipi izliyor.", "path": "report.kept"})
    left = len(report["skipped"]) + len(report["refused"])
    if left:
        out.append({"code": "element_type_skipped",
                    "message": f"{left} eleman güncellenmedi (kilitli katmanda {len(report['skipped'])}, geçersiz olan "
                               f"{len(report['refused'])}); eski sürümde kaldı. Tipi yeniden uygula ile alırlar.",
                    "path": "report.skipped"})
    return out


def step(d, op, inp, result, changed=(), command=None):
    s = {"op": op, "input": inp, "result": result, "within": 1e-9}
    if command:
        s["command"] = command
    if result["status"] != "completed" or op != "execute":
        s["expect"] = {"ids": IDS, "revision": "same"}
        return s
    s["expect"] = {"ids": IDS, "settings": {"arch": d.settings_arch()},
                   "entities": {str(i): d.entities[i] for i in changed}, "revision": "changed"}
    return s


def undo(before, changed=()):
    expect = {"settings": {"arch": before.settings_arch()}, "revision": "changed"}
    if changed:
        expect["entities"] = {str(i): before.entities[i] for i in changed}
    return {"op": "undo", "returns": STEP, "expect": expect}


def case(name, steps):
    return {"name": name, "steps": steps}


def refused_case(name, inp, result):
    d = copy.deepcopy(BASE)
    return case(name, [step(d, "validate", inp, {"status": "failed", "error": result["error"]}), step(d, "execute", inp, result)])


def written_case(name, before, after, inp, id, report, changed, plan=True):
    result = done(edited(after, id, report), warnings_of(report))
    steps = []
    if plan:
        steps.append(step(before, "plan", inp, result))
    steps.append(step(after, "execute", inp, result, changed))
    steps.append(undo(before, changed))
    return case(name, steps)


def follow_all(d, t, fields, refused="stop", own=None):
    """The elements following `t` (by the drawing's order) taking it; `own` (id, keep) an element given its own keep.
    Returns (report, changed ids) or a refusal."""
    report, changed = empty(), []
    nxt = {}
    for id in sorted(d.entities):
        e = d.entities[id]
        l = e.get("arch", {}).get("elementType")
        if not l or l["id"] != t["id"]:
            continue
        if id in d.locked:
            report["skipped"].append({"uid": uid(id), "why": "locked",
                                      "message": "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın."})
            continue
        keep = own[1] if own and own[0] == id else l.get("overrides", [])
        g = follow(e, t, keep)
        why = column_problem(g) if t["kind"] == "column" else None
        if why:
            if refused == "stop":
                return refusal("element_type_refused",
                               f"“{t['name']}” tipini izleyen “{uid(id)}” kimlikli {WORDS[t['kind']]} yeni değerlerle geçersiz "
                               f"oluyor: {why} Hiçbir şey yazılmadı; değeri düzeltin, o elemanı tipten ayırın ya da "
                               f"onRefused: skip ile onu eski sürümde bırakın.", "operation"), None
            report["refused"].append({"uid": uid(id), "why": "invalid", "message": why})
            continue
        kept = [f for f in fields if f in keep]
        if kept:
            report["kept"].append({"uid": uid(id), "fields": kept})
        report["applied"].append(uid(id))
        nxt[id] = g
        changed.append(id)
    d.entities.update(nxt)
    return report, changed


def changed_fields(old, new):
    out = []
    for key, _, _, named in FIELDS[new["kind"]]:
        a = old["name"] if named else old["values"].get(key)
        b = new["name"] if named else new["values"].get(key)
        if a != b:
            out.append(key)
    return out


def update_case(name, key, values, refused="stop", plan=True):
    before = copy.deepcopy(BASE)
    after = copy.deepcopy(BASE)
    inp = {"operation": {"kind": "update", "type": key, "values": values}}
    if refused != "stop":
        inp["onRefused"] = refused
    t = next(x for x in after.types if x["id"] == key.lower() or x["name"].lower() == key.lower())
    old = copy.deepcopy(t)
    vals = dict(t["values"])
    for k, v in values.items():
        if v is None:
            vals.pop(k, None)
        else:
            vals[k] = v
    t["values"] = canonical(t["kind"], vals)
    t["version"] += 1
    report, changed = follow_all(after, t, changed_fields(old, t), refused)
    if changed is None:
        return refused_case(name, inp, report)
    return written_case(name, before, after, inp, t["id"], report, changed, plan)


# ── Cases ────────────────────────────────────────────────────────────────────────────────────────────────────────


def cases():
    out = []
    # The acceptance (plan/bim-plan.md BIM-12): S1's width changes; its 16 columns follow, the one with its own width
    # keeps it; the one on a locked layer stays at the older version.
    out.append(update_case("Kabul: S1'in eni değişir; izleyen kolonlar alır, kendi eni olan korur, kilitli katmandaki "
                           "eski sürümde kalır", "S1", {"width": 0.35}))
    out.append(update_case("Tipi değiştir: değer kaldırılır (null); kolonların boyu enleri olur", "s1", {"depth": None}))
    out.append(update_case("Tipi değiştir: dolgu ve malzeme eklenir", "S1", {"fill": False, "material": "C30"}))
    out.append(update_case("Tipi değiştir: kolu kendi olan L kolon geçersiz olur, bütün değişiklik reddedilir", "L1",
                           {"width": 0.3, "depth": 0.3}))
    out.append(update_case("Tipi değiştir: atla ile geçersiz olan L kolon eski sürümde kalır", "L1",
                           {"width": 0.3, "depth": 0.3}, refused="skip"))
    out.append(update_case("Kiriş tipi değişir; kirişi izler", "K1", {"height": 0.6, "material": "C30"}))

    # Define.
    def define_case(name, op, follower=None):
        before = copy.deepcopy(BASE)
        after = copy.deepcopy(BASE)
        kind = op.get("element")
        values = {}
        if "copyOf" in op:
            src = next(t for t in after.types if t["id"] == op["copyOf"].lower() or t["name"].lower() == op["copyOf"].lower())
            kind, values = src["kind"], dict(src["values"])
        if "from" in op:
            e = after.entities[follower]
            kind = e["kind"]
            values = {k: e[k] for k, _, _, named in FIELDS[kind] if not named and k in e}
        for k, v in op.get("values", {}).items():
            if v is None:
                values.pop(k, None)
            else:
                values[k] = v
        id = op.get("id") or id_of(op["name"])
        n = 2
        base = id
        while any(t["id"] == id for t in after.types):
            id = f"{base}-{n}"
            n += 1
        t = {"id": id, "name": op["name"], "kind": kind, "version": 1, "values": canonical(kind, values)}
        after.types.append(t)
        report, changed = empty(), []
        if follower is not None:
            after.entities[follower] = follow(after.entities[follower], t, [])
            report["applied"].append(uid(follower))
            changed.append(follower)
        return written_case(name, before, after, {"operation": op}, id, report, changed)

    out.append(define_case("Yeni tip: kolon tipi değerleriyle", {"kind": "define", "name": "S2", "element": "column",
                                                                 "values": {"width": 0.4, "depth": 0.4}}))
    out.append(define_case("Yeni tip: varsayılan değer yazılmaz, tam sayı gerçek sayı olur",
                           {"kind": "define", "name": "S3", "element": "column",
                            "values": {"width": 1, "fill": False, "shape": "rect"}}))
    out.append(define_case("Yeni tip: kiriş tipinin kopyası; kimlik adından, Türkçe harfler katlanır",
                           {"kind": "define", "name": "Kiriş Ğ 30", "copyOf": "K1", "values": {"width": 0.3}}))
    out.append(define_case("Yeni tip: kimliği verilen döşeme tipi", {"kind": "define", "name": "Döşeme 20", "id": "doseme-20",
                                                                     "element": "slab",
                                                                     "values": {"thickness": 0.2, "material": "Betonarme"}}))
    out.append(define_case("Yeni tip: aynı ad başka türde olur; kimlik -2 alır",
                           {"kind": "define", "name": "S1", "element": "beam", "values": {"width": 0.2, "height": 0.4}}))
    out.append(define_case("Tipe çevir: çizilmiş kolon tip olur ve onu izler",
                           {"kind": "define", "name": "S4", "from": UID(20)}, follower=20))
    out.append(define_case("Tipe çevir: döşeme", {"kind": "define", "name": "D20", "from": UID(24)}, follower=24))
    out.append(define_case("Tipe çevir: korkuluk", {"kind": "define", "name": "KR1", "from": UID(25)}, follower=25))
    out.append(define_case("Tipe çevir: merdiven", {"kind": "define", "name": "M1", "from": UID(26)}, follower=26))
    out.append(define_case("Tipe çevir: çatı (değeri olmayan tip)", {"kind": "define", "name": "Ç1", "from": UID(27)},
                           follower=27))

    # Rename: the columns take the name as their type; the locked one stays.
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.types[0]["name"] = "S1-30/50"
    report, changed = empty(), []
    for id in sorted(after.entities):
        e = after.entities[id]
        l = e.get("arch", {}).get("elementType")
        if l and l["id"] == "s1" and id not in after.locked and "type" not in l.get("overrides", []):
            e["type"] = "S1-30/50"
            report["applied"].append(uid(id))
            changed.append(id)
    out.append(written_case("Yeniden adlandır: izleyen kolonlar tipin adını alır", before, after,
                            {"operation": {"kind": "rename", "type": "S1", "name": "S1-30/50"}}, "s1", report, changed))

    # Remove.
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.types = [t for t in after.types if t["id"] != "d15"]
    out.append(written_case("Sil: izleyeni olmayan tip", before, after, {"operation": {"kind": "remove", "type": "D15"}},
                            "d15", empty(), []))
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.types = [t for t in after.types if t["id"] != "k1"]
    del after.entities[23]["arch"]["elementType"]
    report = empty()
    report["applied"] = [uid(23)]
    out.append(written_case("Sil: tipten ayırarak; kiriş değerleriyle kalır", before, after,
                            {"operation": {"kind": "remove", "type": "K1", "detach": True}}, "k1", report, [23]))

    # Assign, detach, reset, push, refresh.
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.entities[20] = follow(after.entities[20], after.type("s1"), [])
    report = empty()
    report["applied"] = [uid(20)]
    out.append(written_case("Tipe bağla: kendi kolonu S1'in bütün değerlerini alır", before, after,
                            {"operation": {"kind": "assign", "type": "S1", "uids": [UID(20)]}}, "s1", report, [20]))
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    del after.entities[17]["arch"]["elementType"]
    report = empty()
    report["applied"] = [uid(17)]
    out.append(written_case("Tipten ayır: değerleri kalır, tipi izlemez", before, after,
                            {"operation": {"kind": "detach", "uids": [UID(17)]}}, None, report, [17]))
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.entities[17] = follow(after.entities[17], after.type("s1"), [])
    report = empty()
    report["applied"] = [uid(17)]
    out.append(written_case("Tipe döndür: kendi eni bırakılır", before, after,
                            {"operation": {"kind": "reset", "uids": [UID(17)]}}, None, report, [17]))
    # Push: column 17's own width becomes S1's; the others follow, the locked one stays behind.
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    t = after.type("s1")
    old = copy.deepcopy(t)
    t["values"] = canonical("column", dict(t["values"], width=0.4))
    t["version"] += 1
    report, changed = follow_all(after, t, changed_fields(old, t), own=(17, []))
    out.append(written_case("Tipe uygula: kendi eni tipin olur, öbür kolonlar izler", before, after,
                            {"operation": {"kind": "push", "uid": UID(17)}}, "s1", report, changed))
    # Refresh: the open column at version 1 takes version 2; the locked one is skipped.
    before, after = copy.deepcopy(BASE), copy.deepcopy(BASE)
    after.entities[19] = follow(after.entities[19], after.type("s1"), [])
    report = empty()
    report["skipped"].append({"uid": uid(18), "why": "locked",
                              "message": "Kilitli katmanda; eski sürümde kaldı. Kilidi açıp tipi yeniden uygulayın."})
    report["applied"].append(uid(19))
    out.append(written_case("Tipi yeniden uygula: eski sürümdeki kolon son sürümü alır, kilitli olan kalır", before, after,
                            {"operation": {"kind": "refresh"}}, None, report, [19]))

    # An edit of another command: column 2's width its own, an exception; set back, it follows again.
    d = copy.deepcopy(BASE)
    first = copy.deepcopy(d)
    e = d.entities[2]
    e["width"] = 0.45
    e["arch"]["elementType"]["overrides"] = ["width"]
    edit = {"uids": [UID(2)], "operation": {"kind": "set", "width": 0.45}}
    s1 = step(d, "execute", edit, done({"elements": [e], "revision": "$current"}), [2], command="cad.arch.edit")
    second = copy.deepcopy(d)
    e = copy.deepcopy(e)
    e["width"] = 0.3
    del e["arch"]["elementType"]["overrides"]
    d.entities[2] = e
    back = {"uids": [UID(2)], "operation": {"kind": "set", "width": 0.3}}
    s2 = step(d, "execute", back, done({"elements": [e], "revision": "$current"}), [2], command="cad.arch.edit")
    out.append(case("Başka bir düzenleme alanı istisna yapar; tipin değerine dönünce yine izler", [s1, s2]))
    del first, second

    # Refusals.
    known = ", ".join(k for k, _, _, named in FIELDS["column"] if not named)
    there = ", ".join(f"{t['name']} ({WORDS[t['kind']]})" for t in BASE.types)
    out.append(refused_case("Ret: tip bulunamadı", {"operation": {"kind": "update", "type": "S99", "values": {"width": 0.3}}},
                            refusal("element_type_not_found",
                                    f"“S99” adında ya da kimliğinde eleman tipi yok. Var olanlar: {there}.", "operation.type")))
    out.append(refused_case("Ret: aynı adda kolon tipi var (büyük küçük harf ayrılmaz)",
                            {"operation": {"kind": "define", "name": "s1", "element": "column", "values": {"width": 0.3}}},
                            refusal("element_type_exists", "Projede “S1” adında bir kolon tipi zaten var; başka bir ad verin.",
                                    "operation.name")))
    out.append(refused_case("Ret: türü verilmeyen tip", {"operation": {"kind": "define", "name": "X1"}},
                            refusal("invalid_element_type",
                                    "Tipin türü verilmeli (element: column, beam, slab, roof, railing ya da stair), ya da bir "
                                    "tipin kopyası (copyOf) ya da çizilmiş bir eleman (from).", "operation.element")))
    out.append(refused_case("Ret: zorunlu alan eksik", {"operation": {"kind": "define", "name": "S9", "element": "column",
                                                                       "values": {"depth": 0.5}}},
                            refusal("invalid_element_type", "Kolon tipi En (width) değerini vermeli.",
                                    "operation.values.width")))
    out.append(refused_case("Ret: türün alanı olmayan değer", {"operation": {"kind": "update", "type": "S1",
                                                                              "values": {"tread": 0.3}}},
                            refusal("invalid_element_type",
                                    f"“tread” kolon tipinin alanı değil; tip yalnız şunları verir: {known}.",
                                    "operation.values.tread")))
    out.append(refused_case("Ret: değişen değer yok", {"operation": {"kind": "update", "type": "S1",
                                                                      "values": {"width": 0.3, "fill": True}}},
                            refusal("nothing_to_set", "“S1” tipinin değerleri değişmedi; değişecek bir değer verin.",
                                    "operation.values")))
    out.append(refused_case("Ret: izleyeni olan tip silinmez", {"operation": {"kind": "remove", "type": "S1"}},
                            refusal("element_type_in_use",
                                    "“S1” tipini 19 eleman izliyor; önce onları tipten ayırın ya da silerken ayırmayı seçin "
                                    "(detach).", "operation.type")))
    out.append(refused_case("Ret: kilitli katmandaki izleyen tipten ayrılamaz", {"operation": {"kind": "remove", "type": "S1",
                                                                                              "detach": True}},
                            refusal("layer_locked", "“Kilitli kolonlar” katmanı kilitli; üzerindeki eleman düzenlenemez. "
                                                    "Kilidi Katmanlar panelinden açın.", "operation.type")))
    out.append(refused_case("Ret: kolon tipi döşemeye bağlanmaz", {"operation": {"kind": "assign", "type": "S1",
                                                                                  "uids": [UID(24)]}},
                            refusal("element_type_kind", f"“S1” bir kolon tipi; “{uid(24)}” kimlikli eleman döşeme.",
                                    "uids[0]")))
    out.append(refused_case("Ret: çizgi tip almaz", {"operation": {"kind": "assign", "type": "S1", "uids": [UID(28)]}},
                            refusal("element_type_kind",
                                    f"“{uid(28)}” kimlikli nesne kolon, kiriş, döşeme, çatı, korkuluk ya da merdiven değil; eleman "
                                    "tipi yalnız bunlara verilir.", "uids[0]")))
    out.append(refused_case("Ret: tipsiz eleman tipten ayrılmaz", {"operation": {"kind": "detach", "uids": [UID(20)]}},
                            refusal("element_not_typed", f"“{uid(20)}” kimlikli eleman bir proje tipini izlemiyor.", "uids[0]")))
    out.append(refused_case("Ret: kilitli katmandaki eleman tipe döndürülmez", {"operation": {"kind": "reset",
                                                                                            "uids": [UID(18)]}},
                            refusal("layer_locked", "“Kilitli kolonlar” katmanı kilitli; üzerindeki eleman düzenlenemez. "
                                                    "Kilidi Katmanlar panelinden açın.", "uids[0]")))
    out.append(refused_case("Ret: istisnası olmayan eleman tipe uygulanmaz", {"operation": {"kind": "push", "uid": UID(1)}},
                            refusal("nothing_to_set",
                                    f"“{uid(1)}” kimlikli elemanın “S1” tipinden ayrılan değeri yok; tipe uygulanacak bir şey "
                                    "yok.", "operation.uid")))
    out.append(refused_case("Ret: son sürümdeki eleman yeniden uygulanmaz", {"operation": {"kind": "refresh",
                                                                                          "uids": [UID(1)]}},
                            refusal("nothing_to_set", "Tipinin eski sürümünde eleman yok; hepsi son sürümde.", "uids")))
    out.append(refused_case("Ret: tipten ayrılacak eleman verilmedi", {"operation": {"kind": "detach", "uids": []}},
                            refusal("no_entities", "Tipten ayrılacak eleman verilmedi. En az bir nesnenin kalıcı kimliğini "
                                                   "verin.", "uids")))
    out.append(refused_case("Ret: kimlik UUID değil", {"operation": {"kind": "assign", "type": "S1", "uids": ["kolon"]}},
                            refusal("invalid_uid", "“kolon” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir "
                                                   "UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi "
                                                   "oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]")))
    out.append(refused_case("Ret: çizimde olmayan eleman", {"operation": {"kind": "reset",
                                                                          "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]}},
                            refusal("entity_not_found", "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: "
                                                        "silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin "
                                                        "kimliğini verin.", "uids[0]")))
    out.append(refused_case("Ret: geçersiz beklenen sürüm", {"operation": {"kind": "update", "type": "S1",
                                                                           "values": {"width": 0.35}},
                                                             "expectedRevision": "x1"},
                            refusal("invalid_revision", "Beklenen sürüm “x1” geçerli bir sürüm değil; sürüm “12” gibi bir "
                                                        "tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                                    "expectedRevision")))
    conflict = {"status": "conflict", "error": {"code": "revision_conflict",
                                                "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. "
                                                           "Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                                "path": "expectedRevision", "revision": "$current"}}
    inp = {"operation": {"kind": "update", "type": "S1", "values": {"width": 0.35}}, "expectedRevision": "999"}
    out.append(case("Ret: çizim başka sürümde", [step(copy.deepcopy(BASE), "execute", inp, conflict)]))
    out.append(refused_case("Ret: kopya ile çizilmiş eleman birlikte verilmez",
                            {"operation": {"kind": "define", "name": "S9", "copyOf": "S1", "from": UID(20)}},
                            refusal("invalid_element_type",
                                    "Yeni tip ya bir tipin kopyasıdır (copyOf) ya da çizilmiş bir elemandan (from); ikisi "
                                    "birden verilmez.", "operation.from")))
    return out


def files():
    note = ("ADR 8106. Beklenenler scripts/fixtures/element_types_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
            "olmadan. Kurulumda S1 kolon tipi (sürüm 2: 30/50, dolu), on yedi kolonu (on yedincinin kendi eni 0,40), S1'in "
            "1. sürümünde biri kilitli katmanda iki kolon (boyları 0,60), tipsiz bir kolon, L1 kolon tipi ve iki kolonu (birinin "
            "kendi kolları 0,35), K1 kiriş tipi ve kirişi, D15 döşeme tipi; tipsiz döşeme, korkuluk, merdiven, çatı ve bir çizgi var.")
    return {"cad.elementtypes.edit.json": {
        "format": "kentos.command-cases", "version": 1, "command": "cad.elementtypes.edit", "commandVersion": 1,
        "title": "Eleman tipleri: tanım, tipe çevir, değişiklik ve izleyen elemanlar, istisna, ad, silme, bağlama; retler",
        "note": note, "setup": setup(), "cases": cases()}}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']} ({name}): {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
