#!/usr/bin/env python3
"""The shared cases of dynamic blocks (docs/adr/0318, 0319; plan/cad.md CAD-BLOCK-14, 15, 16, 19).

    python3 scripts/fixtures/dynamic_block_cases.py           # writes the files
    python3 scripts/fixtures/dynamic_block_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.blocks.parameters.json (get, set, reset; Blok ekle's values) and
cad.blocks.edit.dynamic.json (a definition's parameters and actions, their rules, the inserts' values refitted),
cad.blocks.proxy.json (what an insert draws as plain objects) and cad.blocks.freeze.json (Sabit bloğa çevir), with
Python's standard library and no KentOS code. The rules are written here from the ADR on their own:

- an insert keeps only the values that differ from the defaults: a linear's default is the length of its line, an
  angle's its base, a point's and a polar's their points, a flip off, a visibility its first state; a lookup keeps none;
- a number meets its rule within a hair (1e-9 of its size): within min and max, on steps from min (or 0), one of the
  list;
- setting: each name the block's (`unknown_parameter`), not an expression's (`derived_parameter`), a value of its
  kind meeting its rule (`invalid_parameter_value`); a lookup's row sets its values; then every expression is computed
  from the number parameters (`@name`) as many times as there are expressions; the defaults dropped;
- a definition's change keeps the values that still fit, computes expressions again and drops the defaults.
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"


def P(x, y):
    return {"x": x, "y": y}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def num(x):
    """A number as the messages write it: Rust's shortest form, a whole number without its point."""
    return str(int(x)) if float(x).is_integer() else repr(float(x))


def fold(text):
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in text)


KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d8001"
RAF = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d8002"
MASA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d8003"
MISSING = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d8fff"


def line(i, a, b):
    return {"kind": "line", "id": i, "layerId": "", "attrs": {}, "a": a, "b": b}


QUARTER = math.pi / 2
DOOR_DYNAMIC = {
    "parameters": [
        {"kind": "linear", "name": "Genişlik", "from": P(0.0, 0.0), "to": P(0.9, 0.0), "rule": {"min": 0.6, "max": 1.2, "step": 0.05}},
        {"kind": "flip", "name": "Yön", "a": P(0.45, 0.0), "b": P(0.45, 1.0)},
        {"kind": "visibility", "name": "Görünüş", "states": [{"name": "Açılım"}, {"name": "Sade", "hidden": [2]}]},
        {"kind": "lookup", "name": "Tip", "rows": [{"label": "Dar", "values": {"Genişlik": 0.7}},
                                                   {"label": "Standart", "values": {"Genişlik": 0.9}},
                                                   {"label": "Geniş", "values": {"Genişlik": 1.2}}]},
        {"kind": "linear", "name": "Kanat", "from": P(0.0, 0.0), "to": P(0.0, 0.45), "rule": {"expression": "@Genişlik / 2"}},
    ],
    "actions": [
        {"kind": "scale", "parameter": "Genişlik", "pieces": [1, 2]},
        {"kind": "move", "parameter": "Genişlik", "pieces": [4]},
        {"kind": "flip", "parameter": "Yön", "pieces": [1, 2, 3, 4]},
    ],
}
SHELF_DYNAMIC = {
    "parameters": [
        {"kind": "linear", "name": "Uzunluk", "from": P(0.0, 0.0), "to": P(1.0, 0.0), "rule": {"min": 0.5, "max": 3.0, "step": 0.5}},
        {"kind": "angle", "name": "Açı", "center": P(0.0, 0.0), "base": 0.0, "rule": {"list": [0.0, QUARTER]}},
        {"kind": "point", "name": "Etiket", "at": P(0.0, 0.5)},
    ],
    "actions": [
        {"kind": "array", "parameter": "Uzunluk", "pieces": [1], "spacing": 0.5},
        {"kind": "rotate", "parameter": "Açı", "pieces": [1, 2]},
        {"kind": "move", "parameter": "Etiket", "pieces": [2]},
    ],
}
BLOCKS = [
    {"id": KAPI, "name": "Kapı", "base": P(0, 0), "entities": [
        line(1, P(0, 0), P(0, 0.9)),
        {"kind": "arc", "id": 2, "layerId": "", "attrs": {}, "c": P(0, 0), "r": 0.9, "a0": 0, "a1": QUARTER},
        line(3, P(0, -0.1), P(0, 0)), line(4, P(0.9, -0.1), P(0.9, 0)),
    ], "dynamic": DOOR_DYNAMIC},
    {"id": RAF, "name": "Raf", "base": P(0, 0), "entities": [
        line(1, P(0, 0), P(0.4, 0)),
        {"kind": "circle", "id": 2, "layerId": "", "attrs": {}, "c": P(0, 0.5), "r": 0.05},
    ], "dynamic": SHELF_DYNAMIC},
    {"id": MASA, "name": "Masa", "base": P(0, 0), "entities": [line(1, P(-0.6, 0), P(0.6, 0))]},
]


def insert(i, layer_id, block, p, params=None):
    e = {"kind": "insert", "id": i, "layerId": layer_id, "attrs": {}, "block": block, "p": p, "scale": 1, "rotation": 0}
    if params:
        e["parameters"] = params
    return e


ENTITIES = [
    insert(1, "kapilar", KAPI, P(0, 0)),
    insert(2, "kapilar", KAPI, P(5, 0), {"Genişlik": 1.0, "Kanat": 0.5, "Yön": True}),
    insert(3, "kilitli", KAPI, P(10, 0), {"Görünüş": "Sade"}),
    insert(4, "yapi", RAF, P(0, 5)),
    insert(5, "yapi", MASA, P(0, 10)),
    {"kind": "line", "id": 6, "layerId": "yapi", "attrs": {}, "a": P(0, 20), "b": P(5, 20)},
]
IDS = [e["id"] for e in ENTITIES]
LAYERS = [layer("yapi", "Yapı"), layer("kapilar", "Kapılar"), layer("kilitli", "Kilitli", locked=True)]
SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "drawingFont": "barlow"}
SETUP = {"format": "kentos.document", "version": 1, "name": "Dinamik bloklar", "settings": SETTINGS, "origin": P(0, 0),
         "layers": LAYERS, "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []}, "blocks": BLOCKS}


def E(i):
    return copy.deepcopy(next(e for e in ENTITIES if e["id"] == i))


def uid(i):
    return f"$uidOf:{i}"


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


NOTHING = {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}
LOCKED_SKIP = {"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}


# ── The rules ────────────────────────────────────────────────────────────


def slack(x):
    return 1e-9 * max(abs(x), 1.0)


def dist(a, b):
    return math.hypot(b["x"] - a["x"], b["y"] - a["y"])


def default(p):
    k = p["kind"]
    if k == "linear":
        return dist(p["from"], p["to"])
    if k == "polar":
        return p["to"]
    if k == "angle":
        return p["base"]
    if k == "point":
        return p["at"]
    if k == "flip":
        return False
    if k == "visibility":
        return p["states"][0]["name"]
    return None


def same(a, b):
    if type(a) in (int, float) and type(b) in (int, float) and type(a) is not bool and type(b) is not bool:
        return abs(a - b) <= slack(a)
    if type(a) is dict and type(b) is dict:
        return abs(a["x"] - b["x"]) <= slack(a["x"]) and abs(a["y"] - b["y"]) <= slack(a["y"])
    return a == b and type(a) is type(b)


def number_problem(rule, v):
    if "min" in rule and v < rule["min"] - slack(rule["min"]):
        return f"en küçük değer {num(rule['min'])}"
    if "max" in rule and v > rule["max"] + slack(rule["max"]):
        return f"en büyük değer {num(rule['max'])}"
    if "step" in rule:
        start = rule.get("min", 0.0)
        k = (v - start) / rule["step"]
        if abs(k - round(k)) * rule["step"] > slack(v):
            return f"{num(start)} değerinden {num(rule['step'])} adımlarla"
    if rule.get("list") and not any(abs(x - v) <= slack(v) for x in rule["list"]):
        return "şunlardan biri: " + ", ".join(num(x) for x in rule["list"])
    return None


def value_problem(p, v):
    k = p["kind"]
    is_num = type(v) in (int, float) and type(v) is not bool
    if k in ("linear", "angle"):
        return number_problem(p.get("rule", {}), float(v)) if is_num else "bir sayı olmalı"
    if k in ("polar", "point"):
        if type(v) is not dict:
            return "bir nokta ({x, y}) olmalı"
        return None
    if k == "flip":
        return None if type(v) is bool else "açık ya da kapalı (true, false) olmalı"
    if k == "visibility":
        if type(v) is not str:
            return "bir ad olmalı"
        names = [s["name"] for s in p["states"]]
        return None if v in names else "durumlardan biri olmalı: " + ", ".join(names)
    if type(v) is not str:
        return "bir ad olmalı"
    labels = [r["label"] for r in p["rows"]]
    return None if v in labels else "satırlardan biri olmalı: " + ", ".join(labels)


def evaluate(source, values):
    """The few expressions the cases use: `@a / n` and `@a * n`, from the number parameters' values."""
    name, op, n = source.split()
    assert name.startswith("@")
    x = values[name[1:]]
    return x / float(n) if op == "/" else x * float(n)


def numbers(d, kept):
    out = {}
    for p in d["parameters"]:
        if p["kind"] in ("linear", "angle"):
            out[p["name"]] = float(kept.get(p["name"], default(p)))
    return out


def derive(d, values):
    derived = [p for p in d["parameters"] if "expression" in p.get("rule", {})]
    for _ in derived:
        for p in derived:
            x = evaluate(p["rule"]["expression"], numbers(d, values))
            m = number_problem(p["rule"], x)
            if m:
                return p["name"], f"ifadenin sonucu {num(x)}: {m}"
            values[p["name"]] = x
    return None


def resolve(d, kept, given):
    """The values kept after `given` is set; a refusal (code, path, words) instead."""
    values = dict(kept)
    by_name = {p["name"]: p for p in d["parameters"]}
    for name, v in sorted(given.items()):
        path = f"values.{name}"
        p = by_name.get(name)
        if p is None:
            return None, ("unknown_parameter", path, f"Blokta “{name}” parametresi yok. Parametreler: {', '.join(by_name)}.")
        if "expression" in p.get("rule", {}):
            return None, ("derived_parameter", path, f"“{name}” ifadesiyle hesaplanır; değeri verilmez. Hesaplandığı parametreleri değiştirin.")
        m = value_problem(p, v)
        if m:
            return None, ("invalid_parameter_value", path, f"“{name}” parametresinin değeri: {m}.")
        if p["kind"] == "lookup":
            row = next(r for r in p["rows"] if r["label"] == v)
            values.update(row["values"])
        else:
            values[name] = v
    problem = derive(d, values)
    if problem:
        return None, ("invalid_parameter_value", f"values.{problem[0]}", f"“{problem[0]}”: {problem[1]}.")
    return {k: v for k, v in values.items() if not same(default(by_name[k]), v)}, None


def current(d, kept):
    out = {}
    for p in d["parameters"]:
        if p["kind"] != "lookup":
            out[p["name"]] = kept.get(p["name"], default(p))
    for p in d["parameters"]:
        if p["kind"] == "lookup":
            for row in p["rows"]:
                if all(k in out and same(out[k], v) for k, v in row["values"].items()):
                    out[p["name"]] = row["label"]
                    break
    return dict(sorted(out.items()))


def with_params(e, params):
    e = copy.deepcopy(e)
    if params:
        e["parameters"] = dict(sorted(params.items()))
    else:
        e.pop("parameters", None)
    return e


def block_dynamic(block_id):
    return next(b for b in BLOCKS if b["id"] == block_id)["dynamic"]


# ── cad.blocks.parameters ────────────────────────────────────────────────

params = []


def out(entries, changed=()):
    """The answer: each insert's values, the inserts written."""
    return {"inserts": [{"uid": uid(i), "block": block, "values": values} for i, block, values in entries],
            "changed": [uid(i) for i in changed], "revision": "$current"}


def set_case(name, slots, given, note=None):
    entries, changes, changed = [], {}, []
    for i in slots:
        e = E(i)
        d = block_dynamic(e["block"])
        kept, refusal = resolve(d, e.get("parameters", {}), given)
        assert refusal is None, refusal
        entries.append((i, e["block"], current(d, kept)))
        if kept != e.get("parameters", {}):
            changed.append(i)
            changes[str(i)] = with_params(e, kept)
    steps = [{"op": "execute", "input": {"operation": "set", "uids": [uid(i) for i in slots], "values": given},
              "result": done(out(entries, changed)),
              "expect": {"entities": changes, "canUndo": True, "revision": "changed"} if changes else {"ids": IDS, "revision": "same"}}]
    if changes:
        steps.append({"op": "undo", "returns": "Blok parametreleri", "expect": {"entities": {k: E(int(k)) for k in changes}}})
    case = {"name": name, "steps": steps}
    if note:
        case["note"] = note
    params.append(case)


d = DOOR_DYNAMIC
params.append({"name": "oku: varsayılanlar, ifadenin değeri ve tabloda uyan satır", "steps": [
    {"op": "execute", "input": {"operation": "get", "uids": [uid(1)]},
     "result": done(out([(1, KAPI, current(d, {}))])), "expect": {"ids": IDS, "revision": "same"}},
]})
params.append({"name": "oku: iki yerleştirme; uymayan tabloda satır yok", "steps": [
    {"op": "execute", "input": {"operation": "get", "uids": [uid(1), uid(2)]},
     "result": done(out([(1, KAPI, current(d, {})), (2, KAPI, current(d, E(2)["parameters"]))])), "expect": {"ids": IDS, "revision": "same"}},
]})
params.append({"name": "oku: rafın açısı ve noktası", "steps": [
    {"op": "execute", "input": {"operation": "get", "uids": [uid(4)]},
     "result": done(out([(4, RAF, current(SHELF_DYNAMIC, {}))])), "expect": {"ids": IDS, "revision": "same"}},
]})
set_case("değiştir: genişlik; ifade yeniden hesaplanır, tek adım ve geri alınır", [1], {"Genişlik": 1.2})
set_case("değiştir: tablonun satırı genişliği yazar", [1], {"Tip": "Dar"})
set_case("değiştir: görünürlük durumu", [1], {"Görünüş": "Sade"})
set_case("değiştir: varsayılana dönen değer tutulmaz", [2], {"Genişlik": 0.9})
set_case("değiştir: iki yerleştirme, zaten öyle olan yazılmaz", [1, 2], {"Yön": True})
set_case("değiştir: rafın uzunluğu, açısı ve etiketin yeri", [4], {"Uzunluk": 1.5, "Açı": QUARTER, "Etiket": P(0.2, 0.8)})
set_case("değiştir: adımın üstündeki değer kayan noktayla da kabul edilir", [1], {"Genişlik": 1.15})


def reset_case(name, slot, names):
    e = E(slot)
    dd = block_dynamic(e["block"])
    kept = dict(e.get("parameters", {}))
    if names is None:
        kept = {}
    else:
        for n in names:
            p = next(p for p in dd["parameters"] if p["name"] == n)
            if p["kind"] == "lookup":
                for row in p["rows"]:
                    for k in row["values"]:
                        kept.pop(k, None)
            else:
                kept.pop(n, None)
    kept, refusal = resolve(dd, kept, {})
    assert refusal is None
    inp = {"operation": "reset", "uids": [uid(slot)]}
    if names is not None:
        inp["names"] = names
    params.append({"name": name, "steps": [
        {"op": "execute", "input": inp, "result": done(out([(slot, e["block"], current(dd, kept))], [slot])),
         "expect": {"entities": {str(slot): with_params(e, kept)}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Blok parametreleri", "expect": {"entities": {str(slot): E(slot)}}},
    ]})


reset_case("sıfırla: bütün değerler", 2, None)
reset_case("sıfırla: adı verilen parametre", 2, ["Yön"])
reset_case("sıfırla: tablo, satırlarının parametrelerini sıfırlar; ifade yeniden hesaplanır", 2, ["Tip"])

params.append({"name": "planla: cevabı verir, yazmaz", "steps": [
    {"op": "plan", "input": {"operation": "set", "uids": [uid(1)], "values": {"Tip": "Geniş"}},
     "result": done(out([(1, KAPI, current(d, resolve(d, {}, {"Tip": "Geniş"})[0]))], [1])), "expect": NOTHING},
]})
params.append({"name": "kilitli katmandaki atlanır ve söylenir", "steps": [
    {"op": "execute", "input": {"operation": "set", "uids": [uid(1), uid(3)], "values": {"Yön": True}},
     "result": done(out([(1, KAPI, current(d, {"Yön": True})), (3, KAPI, current(d, E(3)["parameters"]))], [1]), [LOCKED_SKIP]),
     "expect": {"entities": {"1": with_params(E(1), {"Yön": True}), "3": E(3)}, "canUndo": True, "revision": "changed"}},
]})
params.append({"name": "bütünü kilitliyse reddedilir", "steps": [
    {"op": "execute", "input": {"operation": "set", "uids": [uid(3)], "values": {"Yön": True}},
     "result": failed("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"), "expect": NOTHING},
]})


def refused(name, inp, result):
    params.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": NOTHING}]})


def refusal_of(slot, given):
    e = E(slot)
    _, (code, path, words) = resolve(block_dynamic(e["block"]), e.get("parameters", {}), given)
    return failed(code, words, path)


refused("ret: yerleştirme verilmedi", {"operation": "get", "uids": []},
        failed("no_entities", "Parametreleri okunacak ya da değişecek yerleştirme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
refused("ret: çizimde olmayan nesne", {"operation": "get", "uids": [MISSING]},
        failed("entity_not_found", f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"))
refused("ret: yerleştirme olmayan nesne", {"operation": "get", "uids": [uid(6)]},
        failed("not_insert", "Verilen nesne bir blok yerleştirmesi değil. Dinamik bir bloğun yerleştirmesini verin.", "uids[0]"))
refused("ret: dinamik olmayan blok", {"operation": "get", "uids": [uid(5)]},
        failed("not_dynamic", "“Masa” bloğunun parametresi yok; dinamik değil. Dinamik bir bloğun yerleştirmesini verin.", "uids[0]"))
refused("ret: değer verilmedi", {"operation": "set", "uids": [uid(1)], "values": {}},
        failed("invalid_parameter_value", "Değer verilmedi. values ile parametrelerin adlarını ve değerlerini verin.", "values"))
refused("ret: blokta olmayan parametre", {"operation": "set", "uids": [uid(1)], "values": {"Renk": 1.0}}, refusal_of(1, {"Renk": 1.0}))
refused("ret: ifadeyle hesaplanan parametre", {"operation": "set", "uids": [uid(1)], "values": {"Kanat": 0.5}}, refusal_of(1, {"Kanat": 0.5}))
refused("ret: adımda olmayan değer", {"operation": "set", "uids": [uid(1)], "values": {"Genişlik": 1.02}}, refusal_of(1, {"Genişlik": 1.02}))
refused("ret: aralığın dışındaki değer", {"operation": "set", "uids": [uid(1)], "values": {"Genişlik": 1.3}}, refusal_of(1, {"Genişlik": 1.3}))
refused("ret: türü yanlış değer", {"operation": "set", "uids": [uid(1)], "values": {"Yön": "evet"}}, refusal_of(1, {"Yön": "evet"}))
refused("ret: olmayan görünürlük durumu", {"operation": "set", "uids": [uid(1)], "values": {"Görünüş": "Kapalı"}}, refusal_of(1, {"Görünüş": "Kapalı"}))
refused("ret: tabloda olmayan satır", {"operation": "set", "uids": [uid(1)], "values": {"Tip": "Çift"}}, refusal_of(1, {"Tip": "Çift"}))
refused("ret: listede olmayan açı", {"operation": "set", "uids": [uid(4)], "values": {"Açı": 1.0}}, refusal_of(4, {"Açı": 1.0}))
refused("ret: sıfırlanacak parametre blokta yok", {"operation": "reset", "uids": [uid(1)], "names": ["Renk"]},
        failed("unknown_parameter", "Blokta “Renk” parametresi yok.", "names[0]"))
params.append({"name": "ret: çizim değişmişse hiçbir şey yazılmaz", "steps": [
    {"op": "execute", "input": {"operation": "set", "uids": [uid(1)], "values": {"Yön": True}, "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision", "revision": "$current"}}, "expect": NOTHING},
]})

params.append({"name": "ret: geçersiz kimlik", "steps": [
    {"op": "execute", "input": {"operation": "get", "uids": ["abc"]},
     "result": failed("invalid_uid", "$any", "uids[0]"), "expect": NOTHING},
]})
params.append({"name": "ret: okunamayan sürüm", "steps": [
    {"op": "execute", "input": {"operation": "get", "uids": [uid(1)], "expectedRevision": "yedi"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": NOTHING},
]})


# ── cad.blocks.edit's dynamic ────────────────────────────────────────────

edit = []
MAX = 256


def dynamic_problem(dy, ids):
    """The contract's rules in their order (docs/adr/0318 §1); the words of the first broken one."""
    params_ = dy["parameters"]
    if not params_:
        return "dinamik bloğun en az bir parametresi olmalı"
    names, visibility = set(), 0
    for p in params_:
        name = p["name"]
        if not name.strip() or name.strip() != name:
            return "parametrenin adı boş olamaz ve boşlukla başlayıp bitemez"
        if fold(name) in names:
            return f"“{name}” adlı iki parametre var"
        names.add(fold(name))
        if p["kind"] in ("linear", "polar") and dist(p["from"], p["to"]) == 0:
            return f"“{name}” parametresinin iki noktası aynı"
        if p["kind"] == "flip" and dist(p["a"], p["b"]) == 0:
            return f"“{name}” parametresinin iki noktası aynı"
        if p["kind"] == "visibility":
            visibility += 1
            if visibility > 1:
                return "bir blokta tek görünürlük parametresi olur"
            seen = set()
            for s in p["states"]:
                if not s["name"].strip() or fold(s["name"]) in seen:
                    return f"“{name}” parametresinin durum adları boş olamaz ve bir kez geçer"
                seen.add(fold(s["name"]))
                bad = [h for h in s.get("hidden", []) if h not in ids]
                if bad:
                    return f"“{s['name']}” durumu blokta olmayan {bad[0]} numaralı nesneyi gizliyor"
        r = p.get("rule")
        if r is not None:
            if "min" in r and "max" in r and r["min"] > r["max"]:
                return f"“{name}” parametresinin kuralı: en küçük değer en büyükten büyük"
            if "step" in r and r["step"] <= 0:
                return f"“{name}” parametresinin kuralı: adım artı bir sayı olmalı"
            if "expression" in r and ("list" in r or "step" in r):
                return f"“{name}” parametresinin kuralı: ifadeyle hesaplanan değerin listesi ya da adımı olmaz"
            if "expression" not in r:
                dv = default(p) if p["kind"] != "polar" else dist(p["from"], p["to"])
                m = number_problem(r, float(dv))
                if m:
                    return f"“{name}” parametresinin varsayılan değeri kurala uymuyor ({m})"
    by_name = {p["name"]: p for p in params_}
    for p in params_:
        if p["kind"] != "lookup":
            continue
        for row in p["rows"]:
            for key, v in row["values"].items():
                t = by_name.get(key)
                if t is None or t["kind"] == "lookup":
                    return f"“{row['label']}” satırı olmayan ya da tablo olan “{key}” parametresine değer veriyor"
                if "expression" in t.get("rule", {}):
                    return f"“{key}” ifadeyle hesaplanır; tablo ona değer veremez"
                m = value_problem(t, v)
                if m:
                    return f"“{row['label']}” satırının “{key}” değeri: {m}"
    fits = {"move": ("linear", "polar", "point"), "stretch": ("linear", "polar", "point"), "scale": ("linear", "polar"),
            "array": ("linear",), "rotate": ("angle",), "flip": ("flip",)}
    words_of = {"linear": "doğrusal", "polar": "kutupsal", "angle": "açı", "point": "nokta", "flip": "çevir",
                "visibility": "görünürlük", "lookup": "tablo"}
    acts = {"move": "taşı", "stretch": "esnet", "scale": "ölçekle", "array": "dizi", "rotate": "döndür", "flip": "aynala"}
    for a in dy.get("actions", []):
        p = by_name.get(a["parameter"])
        if p is None:
            return f"eylemin “{a['parameter']}” parametresi blokta yok"
        if p["kind"] not in fits[a["kind"]]:
            return f"{acts[a['kind']]} eylemi {words_of[p['kind']]} parametresiyle çalışmaz"
        seen = set()
        for i in a["pieces"]:
            if i not in ids or i in seen:
                return f"eylemin {i} numaralı nesnesi blokta yok ya da iki kez verilmiş"
            seen.add(i)
        if a["kind"] == "stretch" and not (a["min"]["x"] < a["max"]["x"] and a["min"]["y"] < a["max"]["y"]):
            return "esnetmenin kutusu sonlu ve en küçük köşesi en büyüğünden küçük olmalı"
        if a["kind"] == "array" and not a["spacing"] > 0:
            return "dizinin aralığı artı bir sayı olmalı"
    return None


def edited(changed, report=None):
    o = {"changed": list(changed), "removed": [], "deleted": [], "revision": "$current"}
    if report is not None:
        o["sync"] = report
    return o


def report(inserts, dropped):
    r = {"inserts": inserts, "added": [], "removed": [], "renamed": [], "constant": [], "places": 0, "fields": 0}
    if dropped:
        r["parameters"] = dropped
    return r


def blocks_after(block_id, dynamic):
    out = []
    for b in copy.deepcopy(BLOCKS):
        if b["id"] == block_id:
            if dynamic is None:
                b.pop("dynamic", None)
            else:
                b["dynamic"] = dynamic
        b.pop("id")
        out.append(b)
    return out


def refit(dynamic, kept):
    if dynamic is None:
        return {}
    by_name = {p["name"]: p for p in dynamic["parameters"]}
    fits = {k: v for k, v in kept.items()
            if k in by_name and by_name[k]["kind"] != "lookup" and "expression" not in by_name[k].get("rule", {})
            and value_problem(by_name[k], v) is None}
    values, refusal = resolve(dynamic, fits, {})
    return values if refusal is None else {}


def changed_case(name, block_id, dynamic, note=None):
    ids = {e["id"] for e in next(b for b in BLOCKS if b["id"] == block_id)["entities"]}
    assert dynamic is None or dynamic_problem(dynamic, ids) is None
    changes, dropped, locked = {}, 0, 0
    for e in ENTITIES:
        if e["kind"] != "insert" or e["block"] != block_id or not e.get("parameters"):
            continue
        values = refit(dynamic, e["parameters"])
        if values == e["parameters"]:
            continue
        if e["layerId"] == "kilitli":
            locked += 1
            continue
        dropped += sum(1 for k in e["parameters"] if k not in values)
        changes[str(e["id"])] = with_params(e, values)
    warnings = []
    if locked:
        warnings.append({"code": "layer_locked", "message": f"{locked} yerleştirme kilitli katmanda olduğu için değerleri eşitlenmedi. Katmanın kilidini Katmanlar panelinden açıp Öznitelikleri eşitle'yi yineleyin.", "path": "block"})
    inp = {"operation": "dynamic", "block": block_id}
    if dynamic is not None:
        inp["dynamic"] = dynamic
    steps = [{"op": "execute", "input": inp, "result": done(edited([block_id], report(len(changes), dropped)), warnings),
              "expect": {"blocks": blocks_after(block_id, dynamic), "entities": changes, "canUndo": True, "revision": "changed"}},
             {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": blocks_after(KAPI, DOOR_DYNAMIC), "entities": {k: E(int(k)) for k in changes}}}]
    case = {"name": name, "steps": steps}
    if note:
        case["note"] = note
    edit.append(case)


SIMPLE = {"parameters": [{"kind": "linear", "name": "Boy", "from": P(-0.6, 0.0), "to": P(0.6, 0.0)}],
          "actions": [{"kind": "stretch", "parameter": "Boy", "pieces": [1], "min": P(0.5, -0.1), "max": P(0.7, 0.1)}]}
changed_case("dinamik: düz bloğa parametre ve eylem", MASA, SIMPLE)
changed_case("dinamik: kaldırılınca yerleştirmelerin değerleri düşer, kilitli katmandaki söylenir", KAPI, None)
narrow = copy.deepcopy(DOOR_DYNAMIC)
narrow["parameters"][0]["rule"]["max"] = 0.95
narrow["parameters"][3]["rows"][2]["values"]["Genişlik"] = 0.95
changed_case("dinamik: artık uymayan değer düşer, ifade yeniden hesaplanır", KAPI, narrow)
renamed = copy.deepcopy(DOOR_DYNAMIC)
renamed["parameters"][1]["name"] = "Taraf"
renamed["actions"][2]["parameter"] = "Taraf"
changed_case("dinamik: adı değişen parametrenin değeri düşer", KAPI, renamed)
twice = copy.deepcopy(DOOR_DYNAMIC)
twice["parameters"][4]["rule"]["expression"] = "@Genişlik * 0.5"
changed_case("dinamik: aynı sonucu veren ifade yerleştirmelere dokunmaz", KAPI, twice)


def bad(name, change, path="dynamic", message=None):
    dy = copy.deepcopy(DOOR_DYNAMIC)
    change(dy)
    ids = {1, 2, 3, 4}
    problem = message or f"“Kapı” bloğunun dinamik parametreleri: {dynamic_problem(dy, ids)}."
    assert message or dynamic_problem(dy, ids), name
    edit.append({"name": name, "steps": [
        {"op": "execute", "input": {"operation": "dynamic", "block": KAPI, "dynamic": dy},
         "result": failed("invalid_dynamic_block", problem, path), "expect": NOTHING},
    ]})


bad("ret: parametresiz", lambda dy: dy.update(parameters=[], actions=[]))
bad("ret: aynı adlı iki parametre (Türkçe büyük küçük harf)", lambda dy: dy["parameters"][1].update(name="GENİŞLİK"))
bad("ret: boşlukla başlayan ad", lambda dy: dy["parameters"][1].update(name=" Yön"))
bad("ret: iki noktası aynı", lambda dy: dy["parameters"][0].update(to=P(0.0, 0.0)))
bad("ret: en küçük en büyükten büyük", lambda dy: dy["parameters"][0]["rule"].update(min=1.3))
bad("ret: adım sıfır", lambda dy: dy["parameters"][0]["rule"].update(step=0.0))
bad("ret: varsayılan değer adımda değil", lambda dy: dy["parameters"][0]["rule"].update(step=0.25))
bad("ret: ifadenin listesi olmaz", lambda dy: dy["parameters"][4]["rule"].update(list=[0.45]))
bad("ret: iki görünürlük", lambda dy: dy["parameters"].append({"kind": "visibility", "name": "İkinci", "states": [{"name": "A"}]}))
bad("ret: aynı adlı iki durum", lambda dy: dy["parameters"][2]["states"][1].update(name="AÇILIM"))
bad("ret: durum olmayan nesneyi gizliyor", lambda dy: dy["parameters"][2]["states"][1].update(hidden=[9]))
bad("ret: tablo olmayan parametreye değer veriyor", lambda dy: dy["parameters"][3]["rows"][0]["values"].update(Renk=1.0))
bad("ret: tablo hesaplanan parametreye değer veriyor", lambda dy: dy["parameters"][3]["rows"][0]["values"].update(Kanat=0.3))
bad("ret: tablonun değeri kurala uymuyor", lambda dy: dy["parameters"][3]["rows"][0]["values"].update({"Genişlik": 0.72}))
bad("ret: eylemin parametresi yok", lambda dy: dy["actions"][0].update(parameter="Boy"))
bad("ret: döndür eylemi doğrusal parametreyle çalışmaz", lambda dy: dy["actions"].append({"kind": "rotate", "parameter": "Genişlik", "pieces": [1]}))
bad("ret: dizi eylemi çevir parametresiyle çalışmaz", lambda dy: dy["actions"].append({"kind": "array", "parameter": "Yön", "pieces": [1], "spacing": 0.5}))
bad("ret: eylemin nesnesi blokta yok", lambda dy: dy["actions"][1].update(pieces=[7]))
bad("ret: eylemde iki kez verilen nesne", lambda dy: dy["actions"][0].update(pieces=[1, 1]))
bad("ret: esnetmenin kutusu ters", lambda dy: dy["actions"].append({"kind": "stretch", "parameter": "Genişlik", "pieces": [4], "min": P(1, 1), "max": P(0, 0)}))
bad("ret: dizinin aralığı sıfır", lambda dy: dy["actions"].append({"kind": "array", "parameter": "Genişlik", "pieces": [3], "spacing": 0.0}))
bad("ret: ifade sayı parametresi olmayan adı okuyor", lambda dy: dy["parameters"][4]["rule"].update(expression="@Yön / 2"),
    "dynamic.parameters[4].rule.expression", "“Kapı” bloğunun “Kanat” parametresinin ifadesi: ifadedeki “@Yön” bir sayı parametresi değil.")
bad("ret: ifadenin sonucu kurala uymuyor", lambda dy: dy["parameters"][4]["rule"].update(expression="@Genişlik * 2", max=1.0),
    "dynamic.parameters[4].rule.expression", "“Kapı” bloğunun “Kanat” parametresinin ifadesi: ifadenin sonucu 1.8: en büyük değer 1.")


# ── cad.blocks.proxy and cad.blocks.freeze (docs/adr/0319 §4) ────────────
#
# A variant written out here from the ADR: the actions in order, each moving its objects (local ids) by its
# parameter's change from the default (move: by the moved point; scale: about the linear's start by now / default;
# array: copies `spacing` apart along the line, as many more as the length holds more spacings than the default, new
# local ids past the largest, after the objects; rotate: about the centre by now − base; flip: mirrored in the axis
# when on); then the visibility state's hidden objects left out. A line moves its ends, a circle its centre (its
# radius times the factor), an arc its centre and its ends, its angles taken again (a mirror swaps its ends).

TAU = 2 * math.pi


def affine_apply(m, p):
    a, b, c, d, e, f = m
    return P(a * p["x"] + c * p["y"] + e, b * p["x"] + d * p["y"] + f)


def moved(e, m):
    """`e` with its geometry through the affine `m`."""
    e = copy.deepcopy(e)
    a, b, c, d, _, _ = m
    det = a * d - b * c
    k = math.sqrt(abs(det))
    if e["kind"] == "line":
        e["a"], e["b"] = affine_apply(m, e["a"]), affine_apply(m, e["b"])
    elif e["kind"] == "circle":
        e["c"], e["r"] = affine_apply(m, e["c"]), e["r"] * k
    elif e["kind"] == "arc":
        c0, r = e["c"], e["r"]
        start = affine_apply(m, P(c0["x"] + r * math.cos(e["a0"]), c0["y"] + r * math.sin(e["a0"])))
        end = affine_apply(m, P(c0["x"] + r * math.cos(e["a1"]), c0["y"] + r * math.sin(e["a1"])))
        cc = affine_apply(m, c0)
        ang = lambda q: math.atan2(q["y"] - cc["y"], q["x"] - cc["x"]) % TAU
        e["c"], e["r"] = cc, r * k
        e["a0"], e["a1"] = (ang(end), ang(start)) if det < 0 else (ang(start), ang(end))
    else:
        raise ValueError(e["kind"])
    return e


def change(p, v):
    if p["kind"] == "linear":
        length = dist(p["from"], p["to"])
        t = (v - length) / length
        return P((p["to"]["x"] - p["from"]["x"]) * t, (p["to"]["y"] - p["from"]["y"]) * t)
    if p["kind"] in ("polar", "point"):
        at = p["to"] if p["kind"] == "polar" else p["at"]
        return P(v["x"] - at["x"], v["y"] - at["y"])
    return None


def vary(block, kept):
    """The objects an insert keeping `kept` draws (docs/adr/0319 §4)."""
    dy = block.get("dynamic")
    out = copy.deepcopy(block["entities"])
    if dy is None or not kept:
        return out
    by_name = {p["name"]: p for p in dy["parameters"]}
    value = lambda p: kept.get(p["name"], default(p))
    next_id = max(e["id"] for e in out) + 1
    for act in dy.get("actions", []):
        p = by_name[act["parameter"]]
        v = value(p)
        chosen = [i for i, e in enumerate(out) if e["id"] in act["pieces"]]
        if act["kind"] == "move":
            dd = change(p, v)
            if dd and (dd["x"] or dd["y"]):
                for i in chosen:
                    out[i] = moved(out[i], (1, 0, 0, 1, dd["x"], dd["y"]))
        elif act["kind"] == "scale":
            base = dist(p["from"], p["to"])
            now = v if p["kind"] == "linear" else dist(p["from"], v)
            k = now / base
            if k != 1:
                f = p["from"]
                for i in chosen:
                    out[i] = moved(out[i], (k, 0, 0, k, f["x"] * (1 - k), f["y"] * (1 - k)))
        elif act["kind"] == "array":
            length = dist(p["from"], p["to"])
            held = lambda x: math.floor(x / act["spacing"] + 1e-9)
            n = max(1, min(1000, held(v) - held(length) + 1))
            ux, uy = (p["to"]["x"] - p["from"]["x"]) / length, (p["to"]["y"] - p["from"]["y"]) / length
            originals = [out[i] for i in chosen]
            for k in range(1, n):
                for e in originals:
                    copy_ = moved(e, (1, 0, 0, 1, ux * act["spacing"] * k, uy * act["spacing"] * k))
                    copy_["id"] = next_id
                    next_id += 1
                    out.append(copy_)
        elif act["kind"] == "rotate":
            turn = v - p["base"]
            if turn:
                c, s_ = math.cos(turn), math.sin(turn)
                o = p["center"]
                m = (c, s_, -s_, c, o["x"] - c * o["x"] + s_ * o["y"], o["y"] - s_ * o["x"] - c * o["y"])
                for i in chosen:
                    out[i] = moved(out[i], m)
        elif act["kind"] == "flip":
            if v is True:
                a, b = p["a"], p["b"]
                t = 2 * math.atan2(b["y"] - a["y"], b["x"] - a["x"])
                c, s_ = math.cos(t), math.sin(t)
                m = (c, s_, s_, -c, a["x"] - c * a["x"] - s_ * a["y"], a["y"] - s_ * a["x"] + c * a["y"])
                for i in chosen:
                    out[i] = moved(out[i], m)
    for p in dy["parameters"]:
        if p["kind"] == "visibility":
            state = next(st for st in p["states"] if st["name"] == value(p))
            out = [e for e in out if e["id"] not in state.get("hidden", [])]
    return out


def block_of(block_id):
    return next(b for b in BLOCKS if b["id"] == block_id)


proxy = []
WITHIN = 1e-9


def proxy_entry(i):
    e = E(i)
    b = block_of(e["block"])
    kept = e.get("parameters", {})
    values = current(b["dynamic"], kept) if "dynamic" in b else {}
    return {"uid": uid(i), "block": e["block"], "default": not kept, "values": values, "entities": vary(b, kept)}


def proxy_case(name, slots, note=None):
    case = {"name": name, "steps": [
        {"op": "execute", "input": {"uids": [uid(i) for i in slots]},
         "result": done({"inserts": [proxy_entry(i) for i in dict.fromkeys(slots)]}), "within": WITHIN, "expect": NOTHING},
    ]}
    if note:
        case["note"] = note
    proxy.append(case)


def proxy_of(block_id, kept, i=None):
    """What an insert of `block_id` keeping `kept` draws: a set step's effect checked through the proxy."""
    b = block_of(block_id)
    return {"uid": uid(i), "block": block_id, "default": not kept,
            "values": current(b["dynamic"], kept) if "dynamic" in b else {}, "entities": vary(b, kept)}


proxy_case("varsayılan kapı: tanımın nesneleri olduğu gibi", [1])
proxy_case("genişletilmiş ve çevrilmiş kapı: ölçek, taşıma ve ayna sırayla; yay uçlarıyla yeniden", [2])
proxy_case("görünürlük durumu: gizlenen yay yok", [3], "Kilitli katmandaki yerleştirme okunur: hiçbir şey yazılmaz.")
proxy_case("raf varsayılanıyla", [4])
proxy_case("düz blok: tanımın kendisi, değeri yok", [5])
proxy_case("üç yerleştirme verildiği sırayla", [5, 2, 1])
proxy_case("aynı kimlik iki kez: bir kez", [1, 1])


def proxy_after_set(name, slot, given, note=None):
    e = E(slot)
    b = block_of(e["block"])
    kept, refusal = resolve(b["dynamic"], e.get("parameters", {}), given)
    assert refusal is None, refusal
    case = {"name": name, "steps": [
        {"op": "execute", "command": "cad.blocks.parameters", "input": {"operation": "set", "uids": [uid(slot)], "values": given},
         "result": done(out([(slot, e["block"], current(b["dynamic"], kept))], [slot] if kept != e.get("parameters", {}) else [])),
         "expect": {"entities": {str(slot): with_params(e, kept)}}},
        {"op": "execute", "input": {"uids": [uid(slot)]},
         "result": done({"inserts": [proxy_of(e["block"], kept, slot)]}), "within": WITHIN, "expect": {"revision": "same"}},
    ]}
    if note:
        case["note"] = note
    proxy.append(case)


proxy_after_set("en geniş kapı", 1, {"Genişlik": 1.2})
proxy_after_set("en dar kapı tablonun satırıyla", 1, {"Tip": "Dar"})
proxy_after_set("çevrilmiş kapı: yay ve kanat aynada, yayın uçları yer değiştirir", 1, {"Yön": True})
proxy_after_set("sade kapı: yay gizli", 1, {"Görünüş": "Sade"})
proxy_after_set("sade, geniş ve çevrilmiş kapı", 1, {"Görünüş": "Sade", "Genişlik": 1.1, "Yön": True})
proxy_after_set("rafın uzunluğu 1,5: dizi bir kopya ekler, yeni yerel kimlikle sonda", 4, {"Uzunluk": 1.5})
proxy_after_set("rafın uzunluğu 3: dört kopya", 4, {"Uzunluk": 3.0})
proxy_after_set("raf 0,5: kopya yok", 4, {"Uzunluk": 0.5})
proxy_after_set("raf dik: çizgi ve daire döner, dizinin kopyası dönmez", 4, {"Uzunluk": 1.5, "Açı": QUARTER},
                "Dizinin kopyaları sonraki eylemin parçaları arasında değildir (yerel kimlikleri yeni).")
proxy_after_set("rafın etiketi taşınır", 4, {"Etiket": P(0.25, 0.75)})


def proxy_refused(name, inp, result):
    proxy.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": NOTHING}]})


proxy_refused("ret: yerleştirme verilmedi", {"uids": []},
              failed("no_entities", "Çizdiği nesneler sorulacak yerleştirme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
proxy_refused("ret: çizimde olmayan nesne", {"uids": [MISSING]},
              failed("entity_not_found", f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"))
proxy_refused("ret: yerleştirme olmayan nesne", {"uids": [uid(1), uid(6)]},
              failed("not_insert", "Verilen nesne bir blok yerleştirmesi değil. Bir bloğun yerleştirmesini verin.", "uids[1]"))
proxy_refused("ret: geçersiz kimlik", {"uids": ["abc"]}, failed("invalid_uid", "$any", "uids[0]"))
proxy.append({"name": "planla: aynı cevap, yazmaz", "steps": [
    {"op": "plan", "input": {"uids": [uid(2)]}, "result": done({"inserts": [proxy_entry(2)]}), "within": WITHIN, "expect": NOTHING},
]})


freeze = []
UNLOCKED = [i for i in IDS if i != 3]


def freeze_case(name, slots, given_name=None, note=None, warnings=()):
    """Sabit bloğa çevir: one new block each distinct block and values in the order first given; the inserts point
    at it, their values dropped; the new block draws what they drew (checked through the proxy)."""
    groups = []
    for i in dict.fromkeys(slots):
        e = E(i)
        if e["layerId"] == "kilitli":
            continue
        key = (e["block"], json.dumps(e.get("parameters", {}), sort_keys=True))
        g = next((g for g in groups if g["key"] == key), None)
        if g is None:
            g = {"key": key, "block": e["block"], "kept": e.get("parameters", {}), "members": []}
            groups.append(g)
        g["members"].append(i)
    taken = [fold(b["name"]) for b in BLOCKS]
    for k, g in enumerate(groups):
        src = block_of(g["block"])
        if given_name is not None:
            name = given_name if k == 0 else f"{given_name} {k + 1}"
        else:
            n = 1
            while True:
                name = f"{src['name']} (sabit)" if n == 1 else f"{src['name']} (sabit {n})"
                if fold(name) not in taken:
                    break
                n += 1
        taken.append(fold(name))
        g["name"] = name
    output = {"blocks": [{"block": f"$blockOf:{g['name']}", "name": g["name"], "from": g["block"],
                          "values": current(block_of(g["block"])["dynamic"], g["kept"]), "inserts": [uid(i) for i in g["members"]]}
                         for g in groups], "revision": "$current"}
    changed = {}
    for g in groups:
        for i in g["members"]:
            e = E(i)
            e["block"] = f"$blockOf:{g['name']}"
            e.pop("parameters", None)
            changed[str(i)] = e
    new_blocks = []
    for g in groups:
        src = block_of(g["block"])
        nb = {"name": g["name"], "base": src["base"], "entities": "$any"}
        new_blocks.append(nb)
    inp = {"uids": [uid(i) for i in slots]}
    if given_name is not None:
        inp["name"] = given_name
    steps = [{"op": "execute", "input": inp, "result": done(output, warnings),
              "expect": {"entities": changed, "blockIds": {g["name"]: "new" for g in groups}, "canUndo": True, "revision": "changed"}}]
    # Each new block draws what its inserts drew: their proxy now a plain block's, at its defaults.
    for g in groups:
        i = g["members"][0]
        steps.append({"op": "execute", "command": "cad.blocks.proxy", "input": {"uids": [uid(i)]},
                      "result": done({"inserts": [{"uid": uid(i), "block": f"$blockOf:{g['name']}", "default": True, "values": {},
                                                    "entities": vary(block_of(g["block"]), g["kept"])}]}),
                      "within": WITHIN, "expect": {"revision": "same"}})
    steps.append({"op": "undo", "returns": "Sabit bloğa çevir", "expect": {"entities": {k: E(int(k)) for k in changed}}})
    case = {"name": name, "steps": steps}
    if note:
        case["note"] = note
    freeze.append(case)


freeze_case("genişletilmiş kapı sabit blok olur: adı kaynağınki ve (sabit), yerleştirmenin değerleri kalkar; geri alınır", [2])
freeze_case("varsayılan kapı da sabitlenebilir: tanımın kopyası", [1])
freeze_case("iki ayrı değer iki blok, verildiği sırayla", [2, 1])
freeze_case("raf: dizinin kopyası yeni bloğun nesnesi", [4])
freeze_case("ad verilir", [2], "Kapı 100")
freeze_case("ad verilir: ikinci bloğa 2", [2, 1], "Sabit kapı")
freeze_case("kilitli katmandaki atlanır ve söylenir", [2, 3], None, None, [LOCKED_SKIP])
freeze_case("aynı kimlik iki kez: bir kez", [2, 2])


def freeze_after_set(name, slot, given):
    e = E(slot)
    b = block_of(e["block"])
    kept, refusal = resolve(b["dynamic"], e.get("parameters", {}), given)
    assert refusal is None
    nm = f"{b['name']} (sabit)"
    after = with_params(e, kept)
    frozen = copy.deepcopy(after)
    frozen["block"] = f"$blockOf:{nm}"
    frozen.pop("parameters", None)
    freeze.append({"name": name, "steps": [
        {"op": "execute", "command": "cad.blocks.parameters", "input": {"operation": "set", "uids": [uid(slot)], "values": given},
         "result": done(out([(slot, e["block"], current(b["dynamic"], kept))], [slot])), "expect": {"entities": {str(slot): after}}},
        {"op": "execute", "input": {"uids": [uid(slot)]},
         "result": done({"blocks": [{"block": f"$blockOf:{nm}", "name": nm, "from": e["block"], "values": current(b["dynamic"], kept),
                                     "inserts": [uid(slot)]}], "revision": "$current"}),
         "expect": {"entities": {str(slot): frozen}, "blockIds": {nm: "new"}}},
        {"op": "execute", "command": "cad.blocks.proxy", "input": {"uids": [uid(slot)]},
         "result": done({"inserts": [{"uid": uid(slot), "block": f"$blockOf:{nm}", "default": True, "values": {}, "entities": vary(b, kept)}]}),
         "within": WITHIN, "expect": {"revision": "same"}},
    ]})


freeze_after_set("sade kapı: gizlenen yay yeni blokta yok", 1, {"Görünüş": "Sade"})
freeze_after_set("dik raf", 4, {"Uzunluk": 2.0, "Açı": QUARTER})
freeze.append({"name": "adı alınmışsa sonraki numara: (sabit 2)", "steps": [
    {"op": "execute", "input": {"uids": [uid(2)]},
     "result": done({"blocks": [{"block": "$blockOf:Kapı (sabit)", "name": "Kapı (sabit)", "from": KAPI,
                                 "values": current(DOOR_DYNAMIC, E(2)["parameters"]), "inserts": [uid(2)]}], "revision": "$current"}),
     "expect": {"blockIds": {"Kapı (sabit)": "new"}}},
    {"op": "execute", "input": {"uids": [uid(1)]},
     "result": done({"blocks": [{"block": "$blockOf:Kapı (sabit 2)", "name": "Kapı (sabit 2)", "from": KAPI,
                                 "values": current(DOOR_DYNAMIC, {}), "inserts": [uid(1)]}], "revision": "$current"}),
     "expect": {"blockIds": {"Kapı (sabit 2)": "new"}}},
]})
freeze.append({"name": "planla: yeni bloklar boş kimlikle, yazmaz", "steps": [
    {"op": "plan", "input": {"uids": [uid(2)]},
     "result": done({"blocks": [{"block": "00000000-0000-0000-0000-000000000000", "name": "Kapı (sabit)", "from": KAPI,
                                 "values": current(DOOR_DYNAMIC, E(2)["parameters"]), "inserts": [uid(2)]}], "revision": "$current"}),
     "expect": NOTHING},
]})


def freeze_refused(name, inp, result):
    freeze.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": NOTHING}]})


freeze_refused("ret: yerleştirme verilmedi", {"uids": []},
               failed("no_entities", "Sabit bloğa çevrilecek yerleştirme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
freeze_refused("ret: boş ad", {"uids": [uid(2)], "name": "  "},
               failed("empty_name", "Blok adı boş olamaz; bir ad yazın ya da adı vermeyin (kaynağın adı ve “(sabit)”).", "name"))
freeze_refused("ret: alınmış ad (Türkçe büyük küçük harf ayırmadan)", {"uids": [uid(2)], "name": "MASA"},
               failed("duplicate_block", "Çizimde “MASA” adında bir blok var; başka bir ad verin.", "name"))
freeze_refused("ret: ikinci bloğun adı alınmış", {"uids": [uid(2), uid(1)], "name": "Raf"},
               failed("duplicate_block", "Çizimde “Raf” adında bir blok var; başka bir ad verin.", "name"))
freeze_refused("ret: dinamik olmayan blok", {"uids": [uid(2), uid(5)]},
               failed("not_dynamic", "“Masa” bloğu dinamik değil; zaten sabit. Dinamik bir bloğun yerleştirmesini verin.", "uids[1]"))
freeze_refused("ret: yerleştirme olmayan nesne", {"uids": [uid(6)]},
               failed("not_insert", "Verilen nesne bir blok yerleştirmesi değil. Bir bloğun yerleştirmesini verin.", "uids[0]"))
freeze_refused("ret: çizimde olmayan nesne", {"uids": [MISSING]},
               failed("entity_not_found", f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"))
freeze_refused("ret: bütünü kilitli", {"uids": [uid(3)]},
               failed("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"))
freeze_refused("ret: geçersiz kimlik", {"uids": ["abc"]}, failed("invalid_uid", "$any", "uids[0]"))
freeze.append({"name": "ret: çizim değişmişse hiçbir şey yazılmaz", "steps": [
    {"op": "execute", "input": {"uids": [uid(2)], "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision", "revision": "$current"}}, "expect": NOTHING},
]})
freeze.append({"name": "ret: okunamayan sürüm", "steps": [
    {"op": "execute", "input": {"uids": [uid(2)], "expectedRevision": "yedi"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": NOTHING},
]})


# ── Writing ──────────────────────────────────────────────────────────────


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def text_of(command, title, note, cases):
    out_ = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', f'  "command": "{command}",', '  "commandVersion": 1,',
            f'  "title": {compact(title)},', f'  "note": {compact(note)},', f'  "setup": {compact(SETUP)},', '  "cases": [']
    blocks = []
    for case in cases:
        lines = ["    {", f'      "name": {compact(case["name"])},']
        if "note" in case:
            lines.append(f'      "note": {compact(case["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(step)}" for step in case["steps"]))
        lines += ["      ]", "    }"]
        blocks.append("\n".join(lines))
    out_.append(",\n".join(blocks))
    out_ += ["  ]", "}"]
    return "\n".join(out_) + "\n"


NOTE = "Written by scripts/fixtures/dynamic_block_cases.py from docs/adr/0318; do not edit by hand."
NOTE19 = "Written by scripts/fixtures/dynamic_block_cases.py from docs/adr/0319 §4; do not edit by hand."
FILES = {
    "cad.blocks.parameters.json": text_of("cad.blocks.parameters", "Blok parametreleri: oku, değiştir, sıfırla; Blok ekle'nin değerleri", NOTE, params),
    "cad.blocks.edit.dynamic.json": text_of("cad.blocks.edit", "Blok düzenle: dinamik parametreler ve eylemler, kuralları, yerleştirmelerin değerleri", NOTE, edit),
    "cad.blocks.proxy.json": text_of("cad.blocks.proxy", "Bloğun vekil görünüşü: yerleştirmenin çizdiği düz nesneler", NOTE19, proxy),
    "cad.blocks.freeze.json": text_of("cad.blocks.freeze", "Sabit bloğa çevir: dinamik bloğun yerleştirmeleri düz bloklara", NOTE19, freeze),
}


def main():
    for name, cases in (("parameters", params), ("edit", edit), ("proxy", proxy), ("freeze", freeze)):
        assert len(cases) >= 20, (name, len(cases))
    if "--check" in sys.argv[1:]:
        for name, text in FILES.items():
            if (OUT / name).read_text(encoding="utf-8") != text:
                sys.exit(f"fixtures/commands/v1/{name} güncel değil")
        print(f"{len(params)} + {len(edit)} + {len(proxy)} + {len(freeze)} durum tutarlı")
        return
    for name, text in FILES.items():
        (OUT / name).write_text(text, encoding="utf-8")
    print(f"{len(params)} + {len(edit)} + {len(proxy)} + {len(freeze)} durum yazıldı")


main()
