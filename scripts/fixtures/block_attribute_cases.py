#!/usr/bin/env python3
"""The shared cases of block attributes' options, their sync, block replacement and the block query
(docs/adr/0317 §1–§3; plan/cad.md CAD-BLOCK-08, 09, 10).

    python3 scripts/fixtures/block_attribute_cases.py           # writes the files
    python3 scripts/fixtures/block_attribute_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.blocks.edit.attributes.json (sync, fields, place, card, the options' rules),
cad.blocks.replace.json and cad.blocks.query.json, with Python's standard library and no KentOS code. The rules are
written here from the ADR on their own:

- an insert holds only the values it was given; a sync moves renamed values, takes off those of tags no attribute has
  any more (or named), a constant attribute's own value, places of attributes no longer movable, and fills fields;
  inserts on a locked layer are left as they are and counted;
- a field is a template of literal text and `{TAG}` parts, each the insert's value of that tag (only such fields are
  used here);
- an insert's value of a tag is the definition's when the attribute is constant, else its own, else the default;
- text conditions fold Turkish case (I → ı, İ → i, then lower case); `lt`, `gt` compare numbers;
- a replaced insert keeps the values of the new block's attributes that are not constant and the places of its
  movable ones;
- the inverse of an insert's placement maps a drawing point to its definition (quarter turns are exact).
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


def style(color="ink"):
    return {"color": color, "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": style(), "children": []}


def fold(text):
    """Turkish case folded: I → ı, İ → i, then each character lowercased."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in text)


KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001"
PENCERE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7002"
LAMBA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7003"
GRUP = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7004"
MISSING = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7fff"
NIL = "00000000-0000-0000-0000-000000000000"


def att(tag, p, value=None, prompt=None, **options):
    a = {"tag": tag}
    if prompt is not None:
        a["prompt"] = prompt
    if value is not None:
        a["value"] = value
    a.update({"p": p, "height": 0.2, "rotation": 0})
    a.update(options)
    return a


KAPI_ATTRS = [
    att("NUMARA", P(0.2, 0.5), "K-0", "Kapı numarası", movable=True),
    att("KAT", P(0.2, 0.2), "1"),
    att("ÜRETİCİ", P(0.2, -0.1), "KentOS", invisible=True, constant=True),
    att("ETİKET", P(0.2, -0.4), preset=True, field="No: {NUMARA}"),
]
PENCERE_ATTRS = [att("NUMARA", P(0, 0.3), "P-0"), att("GENİŞLİK", P(0, 0), "1.2")]

BLOCKS = [
    {"id": KAPI, "name": "Kapı", "base": P(0, 0), "entities": [
        {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0, 0.9)},
    ], "attributes": KAPI_ATTRS, "card": {"category": "Mimari/Kapılar", "tags": ["kapı"], "version": 2}},
    {"id": PENCERE, "name": "Pencere", "base": P(0, 0), "entities": [
        {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(1.2, 0)},
    ], "attributes": PENCERE_ATTRS},
    {"id": LAMBA, "name": "Lamba", "base": P(0, 0), "entities": [
        {"kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": P(0, 0), "r": 0.3},
    ]},
    {"id": GRUP, "name": "Kapı grubu", "base": P(0, 0), "entities": [
        {"kind": "insert", "id": 1, "layerId": "", "attrs": {"NUMARA": "G-1"}, "block": KAPI, "p": P(0, 0), "scale": 1, "rotation": 0},
        {"kind": "insert", "id": 2, "layerId": "", "attrs": {"NUMARA": "G-2"}, "block": KAPI, "p": P(2, 0), "scale": 1, "rotation": 0},
    ]},
]


def insert(i, layer_id, block, p, attrs=None, rotation=0, scale=1, places=None):
    e = {"kind": "insert", "id": i, "layerId": layer_id, "attrs": attrs or {}, "block": block, "p": p, "scale": scale, "rotation": rotation}
    if places:
        e["attributePlaces"] = places
    return e


QUARTER = math.pi / 2
ENTITIES = [
    insert(1, "kapilar", KAPI, P(10, 0), {"NUMARA": "K-1", "KAT": "1", "ETİKET": "No: K-1"}, places={"NUMARA": P(0.5, 1.0)}),
    insert(2, "kapilar", KAPI, P(20, 0), {"NUMARA": "K-2", "KAT": "2", "ESKİ": "x"}),
    insert(3, "kilitli", KAPI, P(30, 0), {"NUMARA": "K-3"}),
    insert(4, "yapi", PENCERE, P(40, 0), {"NUMARA": "P-1", "GENİŞLİK": "1.5"}),
    insert(5, "kapilar", PENCERE, P(50, 0), {"NUMARA": "P-2"}, rotation=QUARTER),
    insert(6, "yapi", GRUP, P(60, 0)),
    {"kind": "line", "id": 7, "layerId": "yapi", "attrs": {}, "a": P(0, 10), "b": P(10, 10)},
    insert(8, "kapilar", KAPI, P(70, 0), {"NUMARA": "K-8"}, rotation=QUARTER, scale=2),
]
IDS = [e["id"] for e in ENTITIES]
LAYERS = [layer("yapi", "Yapı"), layer("kapilar", "Kapılar"), layer("kilitli", "Kilitli", locked=True)]
LOCKED = {"kilitli"}
SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "drawingFont": "barlow"}
SETUP = {"format": "kentos.document", "version": 1, "name": "Blok öznitelikleri", "settings": SETTINGS, "origin": P(0, 0),
         "layers": LAYERS, "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []}, "blocks": BLOCKS}


def E(i):
    return copy.deepcopy(next(e for e in ENTITIES if e["id"] == i))


def B(block_id):
    return copy.deepcopy(next(b for b in BLOCKS if b["id"] == block_id))


def without_ids(blocks):
    out = []
    for b in blocks:
        b = copy.deepcopy(b)
        b.pop("id")
        out.append(b)
    return out


def uid(i):
    return f"$uidOf:{i}"


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


NOTHING = {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}
LOCKED_WARNING = lambda n, path="block": {"code": "layer_locked", "message": f"{n} yerleştirme kilitli katmanda olduğu için değerleri eşitlenmedi. Katmanın kilidini Katmanlar panelinden açıp Öznitelikleri eşitle'yi yineleyin.", "path": path}  # noqa: E731


# ── The rules ────────────────────────────────────────────────────────────


def fill(template, attrs):
    """A field: literal text and `{TAG}` parts, each the insert's value of that tag."""
    out, i = "", 0
    while i < len(template):
        if template[i] == "{":
            j = template.index("}", i)
            out += attrs.get(template[i + 1:j], "")
            i = j + 1
        else:
            out += template[i]
            i += 1
    return out


def sync(entities, old, new, renames=None, remove=(), fill_fields=True, only_fields=False):
    """The drawing's inserts of `new` fitted to it (docs/adr/0317 §2): their changes by id, the report, the locked."""
    renames = renames or {}
    tags = {a["tag"]: a for a in new.get("attributes", [])}
    gone = [a["tag"] for a in old.get("attributes", []) if a["tag"] not in tags and a["tag"] not in renames]
    for t in remove:
        if t not in gone:
            gone.append(t)
    removed, renamed, constant, added = set(), set(), set(), set()
    places = fields = locked = 0
    fitted = []
    for e in entities:
        if e["kind"] != "insert" or e["block"] != new["id"]:
            continue
        if e["layerId"] in LOCKED:
            locked += 1
            continue
        i = copy.deepcopy(e)
        changed = False
        if not only_fields:
            for frm, to in sorted(renames.items()):
                if frm in i["attrs"]:
                    i["attrs"][to] = i["attrs"].pop(frm)
                    changed = True
                    renamed.add(frm)
                if frm in i.get("attributePlaces", {}):
                    i["attributePlaces"][to] = i["attributePlaces"].pop(frm)
                    changed = True
            for t in gone:
                if t in i["attrs"]:
                    del i["attrs"][t]
                    changed = True
                    removed.add(t)
            for a in new.get("attributes", []):
                if a.get("constant"):
                    if a["tag"] in i["attrs"]:
                        del i["attrs"][a["tag"]]
                        changed = True
                        constant.add(a["tag"])
                elif a["tag"] not in i["attrs"]:
                    added.add(a["tag"])
            kept = {k: v for k, v in i.get("attributePlaces", {}).items() if tags.get(k, {}).get("movable")}
            if len(kept) != len(i.get("attributePlaces", {})):
                places += len(i["attributePlaces"]) - len(kept)
                changed = True
                if kept:
                    i["attributePlaces"] = kept
                else:
                    i.pop("attributePlaces")
        fitted.append([i, changed])
    if fill_fields:
        for a in new.get("attributes", []):
            if "field" not in a:
                continue
            for pair in fitted:
                text = fill(a["field"], pair[0]["attrs"])
                if pair[0]["attrs"].get(a["tag"]) != text:
                    pair[0]["attrs"][a["tag"]] = text
                    pair[1] = True
                    fields += 1
    changes = {i["id"]: i for i, changed in fitted if changed}
    report = {
        "inserts": len(changes),
        "added": [a["tag"] for a in new.get("attributes", []) if a["tag"] in added],
        "removed": [t for t in gone if t in removed],
        "renamed": [[f, t] for f, t in sorted(renames.items()) if f in renamed],
        "constant": [a["tag"] for a in new.get("attributes", []) if a["tag"] in constant],
        "places": places,
        "fields": fields,
    }
    return changes, report, locked


def placement(p, sx, sy, rotation):
    """An insert's placement (no mirror here): a quarter turn exact."""
    quarters = {0: (0.0, 1.0), QUARTER: (1.0, 0.0)}
    s, c = quarters[rotation]
    return [sx * c, sx * s, -sy * s, sy * c, p["x"], p["y"]]


def definition_point(insert_e, base, at):
    m = placement(insert_e["p"], insert_e["scale"], insert_e.get("yScale", insert_e["scale"]), insert_e["rotation"])
    det = m[0] * m[3] - m[1] * m[2]
    dx, dy = at["x"] - m[4], at["y"] - m[5]
    return P((m[3] * dx - m[2] * dy) / det + base["x"], (m[0] * dy - m[1] * dx) / det + base["y"])


def value_of(block, e, tag):
    a = next((a for a in block.get("attributes", []) if a["tag"] == tag), None)
    if a is None:
        return ""
    if a.get("constant"):
        return a.get("value", "")
    return e["attrs"].get(tag, a.get("value", ""))


def number(text):
    try:
        x = float(text.strip())
    except ValueError:
        return None
    return x if math.isfinite(x) else None


def meets(value, f):
    want = f.get("value", "")
    op = f["op"]
    if op == "eq":
        return fold(value) == fold(want)
    if op == "ne":
        return fold(value) != fold(want)
    if op == "contains":
        return fold(want) in fold(value)
    if op == "empty":
        return value == ""
    if op == "notEmpty":
        return value != ""
    a, b = number(value), number(want)
    if a is None or b is None:
        return False
    return a < b if op == "lt" else a > b


# ── cad.blocks.edit: sync, fields, place, card, options ─────────────────

edit = []


def edited(changed=(), sync_report=None):
    out = {"changed": list(changed), "removed": [], "deleted": [], "revision": "$current"}
    if sync_report is not None:
        out["sync"] = sync_report
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def blocks_with(*changes):
    out = []
    for b in BLOCKS:
        b = copy.deepcopy(b)
        for block_id, new in changes:
            if b["id"] == block_id:
                b = copy.deepcopy(new)
        out.append(b)
    return without_ids(out)


def entities_expect(changes):
    return {str(i): e for i, e in changes.items()}


def bumped(block, attributes):
    b = copy.deepcopy(block)
    b["attributes"] = attributes
    if "card" in b:
        b["card"]["version"] += 1
    return b


changes, report, locked = sync(ENTITIES, B(KAPI), B(KAPI), remove=["ESKİ"])
assert locked == 1 and report["removed"] == ["ESKİ"] and report["fields"] == 2
edit.append({"name": "eşitle: removeTags değeri yerleştirmeden kaldırır, alanlar dolar, kilitli katmandaki atlanır; tek adım, geri alınır", "steps": [
    {"op": "execute", "input": {"operation": "sync", "block": KAPI, "removeTags": ["ESKİ"]},
     "result": done(edited(sync_report=report), [LOCKED_WARNING(1)]),
     "expect": {"entities": entities_expect(changes), "blocks": without_ids(BLOCKS), "canUndo": True, "revision": "changed"}},
    {"op": "undo", "returns": "Öznitelikleri eşitle", "expect": {"entities": {"2": E(2), "8": E(8)}}},
]})
changes, report, locked = sync(ENTITIES, B(KAPI), B(KAPI), renames={"ESKİ": "KAT"})
edit.append({"name": "eşitle: renames eski etiketin değerini yenisine taşır", "steps": [
    {"op": "execute", "input": {"operation": "sync", "block": KAPI, "renames": {"ESKİ": "KAT"}},
     "result": done(edited(sync_report=report), [LOCKED_WARNING(1)]), "expect": {"entities": entities_expect(changes)}},
]})
changes, report, locked = sync(ENTITIES, B(PENCERE), B(PENCERE))
assert not changes
edit.append({"name": "eşitle: uyan yerleştirmeler yazılmaz, eksik etiketler söylenir", "steps": [
    {"op": "execute", "input": {"operation": "sync", "block": PENCERE}, "result": done(edited(sync_report=report)),
     "expect": {"ids": IDS, "revision": "same"}},
]})
renamed_attrs = [att("NUMARA", P(0.2, 0.5), "K-0", "Kapı numarası", movable=True), att("KATNO", P(0.2, 0.2), "1"),
                 KAPI_ATTRS[2], KAPI_ATTRS[3]]
new = bumped(B(KAPI), renamed_attrs)
changes, report, locked = sync(ENTITIES, B(KAPI), new, renames={"KAT": "KATNO"})
edit.append({"name": "öznitelikler: etiketi değişen değer renames ile taşınır, kart sürümü artar", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": renamed_attrs, "renames": {"KAT": "KATNO"}},
     "result": done(edited([KAPI], report), [LOCKED_WARNING(1)]),
     "expect": {"blocks": blocks_with((KAPI, new)), "entities": entities_expect(changes)}},
    {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": without_ids(BLOCKS), "entities": {"1": E(1), "2": E(2)}}},
]})
dropped_attrs = [KAPI_ATTRS[0], KAPI_ATTRS[2], KAPI_ATTRS[3]]
new = bumped(B(KAPI), dropped_attrs)
changes, report, locked = sync(ENTITIES, B(KAPI), new)
edit.append({"name": "öznitelikler: kaldırılan etiketin değerleri yerleştirmelerden kalkar", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": dropped_attrs},
     "result": done(edited([KAPI], report), [LOCKED_WARNING(1)]),
     "expect": {"blocks": blocks_with((KAPI, new)), "entities": entities_expect(changes)}},
]})
new = bumped(B(KAPI), dropped_attrs)
edit.append({"name": "öznitelikler: sync false yerleştirmelere dokunmaz", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": dropped_attrs, "sync": False},
     "result": done(edited([KAPI])), "expect": {"blocks": blocks_with((KAPI, new)), "entities": {"1": E(1), "2": E(2)}}},
]})
fixed_attrs = [att("NUMARA", P(0.2, 0.5), "K-0", "Kapı numarası")] + KAPI_ATTRS[1:]
new = bumped(B(KAPI), fixed_attrs)
changes, report, locked = sync(ENTITIES, B(KAPI), new)
assert report["places"] == 1
edit.append({"name": "öznitelikler: taşınabilir olmaktan çıkan özniteliğin yeri düşer", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": fixed_attrs},
     "result": done(edited([KAPI], report), [LOCKED_WARNING(1)]),
     "expect": {"blocks": blocks_with((KAPI, new)), "entities": entities_expect(changes)}},
]})
constant_attrs = [KAPI_ATTRS[0], att("KAT", P(0.2, 0.2), "1", constant=True), KAPI_ATTRS[2], KAPI_ATTRS[3]]
new = bumped(B(KAPI), constant_attrs)
changes, report, locked = sync(ENTITIES, B(KAPI), new)
assert report["constant"] == ["KAT"]
edit.append({"name": "öznitelikler: sabit olan özniteliğin yerleştirmedeki değeri kalkar", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": constant_attrs},
     "result": done(edited([KAPI], report), [LOCKED_WARNING(1)]),
     "expect": {"blocks": blocks_with((KAPI, new)), "entities": entities_expect(changes)}},
]})
edit.append({"name": "öznitelikler: birlikte olamayan seçenekler", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": PENCERE, "attributes": [att("NO", P(0, 0), constant=True, verify=True)]},
     "result": failed("invalid_attribute_options", "“Pencere” bloğunun özniteliği: sabit öznitelik sorulmaz; doğrula ve ön ayarlı seçilemez.", "attributes[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "attributes", "block": PENCERE, "attributes": [att("NO", P(0, 0), boxWidth=2.0)]},
     "result": failed("invalid_attribute_options", "“Pencere” bloğunun özniteliği: kutu genişliği yalnız çok satırlı öznitelikte olur.", "attributes[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "attributes", "block": PENCERE, "attributes": [att("NO", P(0, 0), constant=True, movable=True)]},
     "result": failed("invalid_attribute_options", "“Pencere” bloğunun özniteliği: sabit özniteliğin yeri yerleştirmede değişmez; taşınabilir seçilemez.", "attributes[0]"), "expect": NOTHING},
]})
edit.append({"name": "öznitelikler: okunamayan alan", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": PENCERE, "attributes": [att("NO", P(0, 0), field="No: {NUMARA")]},
     "result": failed("invalid_field", "$any", "attributes[0].field"), "expect": NOTHING},
]})
edit.append({"name": "öznitelikler: yeniden adlandırmanın hedefi sabit olmayan bir öznitelik olmalı", "steps": [
    {"op": "execute", "input": {"operation": "attributes", "block": KAPI, "attributes": KAPI_ATTRS, "renames": {"KAT": "ÜRETİCİ"}},
     "result": failed("unknown_tag", "“KAT” → “ÜRETİCİ”: yeni etiket bloğun sabit olmayan bir özniteliği olmalı ve bir kez verilmeli.", "renames.KAT"), "expect": NOTHING},
]})
changes, report, locked = sync(ENTITIES, B(KAPI), B(KAPI), only_fields=True)
assert report["fields"] == 2 and report["removed"] == []
edit.append({"name": "alanlar: alanlı öznitelikler yeniden dolar, başka bir şey değişmez", "steps": [
    {"op": "execute", "input": {"operation": "fields", "block": KAPI},
     "result": done(edited(sync_report=report), [LOCKED_WARNING(1)]), "expect": {"entities": entities_expect(changes)}},
    {"op": "undo", "returns": "Alanları güncelle", "expect": {"entities": {"2": E(2)}}},
]})
edit.append({"name": "alanlar: blok verilmezse alanı olan her blok", "steps": [
    {"op": "execute", "input": {"operation": "fields"},
     "result": done(edited(sync_report={**report, "added": [], "removed": [], "renamed": [], "constant": [], "places": 0}), [LOCKED_WARNING(1)]),
     "expect": {"entities": entities_expect(changes)}},
]})


def placed(i, tag, at):
    e = E(i)
    if at is None:
        e.get("attributePlaces", {}).pop(tag, None)
        if not e.get("attributePlaces"):
            e.pop("attributePlaces", None)
    else:
        e.setdefault("attributePlaces", {})[tag] = definition_point(e, P(0, 0), at)
    return e


PLACE = lambda n: {"inserts": n, "added": [], "removed": [], "renamed": [], "constant": [], "places": 0, "fields": 0}  # noqa: E731
edit.append({"name": "konum: taşınabilir öznitelik çizimdeki noktaya, tanımın koordinatlarında", "steps": [
    {"op": "execute", "input": {"operation": "place", "uids": [uid(2)], "tag": "NUMARA", "at": P(20.5, 1.25)},
     "result": done(edited(sync_report=PLACE(1))), "expect": {"entities": {"2": placed(2, "NUMARA", P(20.5, 1.25))}, "canUndo": True}},
    {"op": "undo", "returns": "Öznitelik konumu", "expect": {"entities": {"2": E(2)}}},
]})
assert placed(8, "NUMARA", P(69, 1))["attributePlaces"]["NUMARA"] == P(0.5, 0.5)
edit.append({"name": "konum: dönük ve ölçekli yerleştirmede tanımın noktası", "steps": [
    {"op": "execute", "input": {"operation": "place", "uids": [uid(8)], "tag": "NUMARA", "at": P(69, 1)},
     "result": done(edited(sync_report=PLACE(1))), "expect": {"entities": {"8": placed(8, "NUMARA", P(69, 1))}}},
]})
edit.append({"name": "konum: at verilmezse tanımın yerine döner", "steps": [
    {"op": "execute", "input": {"operation": "place", "uids": [uid(1)], "tag": "NUMARA"},
     "result": done(edited(sync_report=PLACE(1))), "expect": {"entities": {"1": placed(1, "NUMARA", None)}}},
    {"op": "execute", "input": {"operation": "place", "uids": [uid(1)], "tag": "NUMARA"},
     "result": done(edited(sync_report=PLACE(0))), "expect": {"revision": "same"}},
]})
edit.append({"name": "konum: retler", "steps": [
    {"op": "execute", "input": {"operation": "place", "tag": "NUMARA", "at": P(0, 0)},
     "result": failed("no_entities", "Konumu değişecek yerleştirme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "place", "uids": [uid(2)], "at": P(0, 0)},
     "result": failed("unknown_tag", "Taşınacak özniteliğin etiketi verilmedi. tag ile bloğun taşınabilir bir özniteliğini verin.", "tag"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "place", "uids": [uid(2)], "tag": "KAT", "at": P(0, 0)},
     "result": failed("attribute_not_movable", "“Kapı” bloğunda “KAT” etiketli taşınabilir bir öznitelik yok. Blok öznitelikleri penceresinde özniteliği taşınabilir yapın.", "tag"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "place", "uids": [uid(7)], "tag": "NUMARA", "at": P(0, 0)},
     "result": failed("not_insert", "Verilen nesne bir blok yerleştirmesi değil. Bir bloğun yerleştirmesini verin.", "uids[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "place", "uids": [uid(3)], "tag": "NUMARA", "at": P(0, 0)},
     "result": failed("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"), "expect": NOTHING},
]})
carded = B(PENCERE)
carded["card"] = {"category": "Mimari/Pencereler", "tags": ["pencere", "doğrama"], "version": 1}
carded["description"] = "Tek kanatlı pencere"
edit.append({"name": "kart: kategori, etiketler ve açıklama; ilk kartın sürümü 1", "steps": [
    {"op": "execute", "input": {"operation": "card", "block": PENCERE, "category": "Mimari/Pencereler", "tags": ["pencere", "doğrama"], "description": "Tek kanatlı pencere"},
     "result": done(edited([PENCERE])), "expect": {"blocks": blocks_with((PENCERE, carded)), "canUndo": True}},
    {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": without_ids(BLOCKS)}},
]})
recarded = B(KAPI)
recarded["card"] = {"category": "Mimari/Kapılar", "tags": ["kapı", "giriş"], "version": 2}
edit.append({"name": "kart: kartı değiştirmek sürümü artırmaz; verilmeyen alan kalır", "steps": [
    {"op": "execute", "input": {"operation": "card", "block": KAPI, "tags": ["kapı", "giriş"]},
     "result": done(edited([KAPI])), "expect": {"blocks": blocks_with((KAPI, recarded))}},
]})
edit.append({"name": "kart: aynı etiket iki kez, boşluklu kategori", "steps": [
    {"op": "execute", "input": {"operation": "card", "block": KAPI, "tags": ["Kapı", "KAPI"]},
     "result": failed("invalid_block_card", "“Kapı” bloğunun kartı: “KAPI” etiketi iki kez var.", "tags"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": "card", "block": KAPI, "category": " Mimari"},
     "result": failed("invalid_block_card", "“Kapı” bloğunun kartı: “ Mimari” kategorisi boş ya da başında, sonunda boşluk var.", "category"), "expect": NOTHING},
]})
rebased = B(KAPI)
rebased["base"] = P(0.1, 0)
rebased["card"]["version"] = 3
edit.append({"name": "kart: tanımın çizdiği değişince sürüm artar, adı değişince artmaz", "steps": [
    {"op": "execute", "input": {"operation": "rebase", "block": KAPI, "base": P(0.1, 0)},
     "result": done(edited([KAPI])), "expect": {"blocks": blocks_with((KAPI, rebased))}},
    {"op": "execute", "input": {"operation": "rename", "block": KAPI, "name": "Kapı 90"},
     "result": done(edited([KAPI])), "expect": {"blocks": blocks_with((KAPI, {**rebased, "name": "Kapı 90"}))}},
]})


# ── cad.blocks.replace ───────────────────────────────────────────────────

replace = []


def replaced(frm, to, ids):
    """The inserts `ids` of `frm` placing `to`: the values and places kept, those dropped, the fields filled."""
    old, new = B(frm), B(to)
    kept = {a["tag"] for a in new.get("attributes", []) if not a.get("constant")}
    movable = {a["tag"] for a in new.get("attributes", []) if a.get("movable")}
    dropped, places, fields, out = set(), 0, 0, {}
    for i in ids:
        e = E(i)
        e["block"] = to
        for a in old.get("attributes", []):
            if a["tag"] not in kept and a["tag"] in e["attrs"]:
                del e["attrs"][a["tag"]]
                dropped.add(a["tag"])
        ps = {k: v for k, v in e.get("attributePlaces", {}).items() if k in movable}
        places += len(e.get("attributePlaces", {})) - len(ps)
        if ps:
            e["attributePlaces"] = ps
        else:
            e.pop("attributePlaces", None)
        for a in new.get("attributes", []):
            if "field" in a:
                e["attrs"][a["tag"]] = fill(a["field"], e["attrs"])
                fields += 1
        out[i] = e
    report = {"replaced": [uid(i) for i in ids], "dropped": [a["tag"] for a in old.get("attributes", []) if a["tag"] in dropped],
              "places": places, "fields": fields, "purged": False, "revision": "$current"}
    return out, report


LOCKED_SKIP = {"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}
out, report = replaced(KAPI, PENCERE, [1, 2, 8])
assert report["dropped"] == ["KAT", "ETİKET"] and report["places"] == 1
replace.append({"name": "bütün yerleştirmeler: uyan değerler kalır, düşenler söylenir, kilitli katmandaki atlanır; geri alınır", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE}, "result": done(report, [LOCKED_SKIP]),
     "expect": {"entities": entities_expect(out), "canUndo": True, "revision": "changed"}},
    {"op": "undo", "returns": "Bloğu başkasıyla değiştir", "expect": {"entities": {"1": E(1), "2": E(2), "8": E(8)}}},
]})
out, report = replaced(KAPI, PENCERE, [2])
replace.append({"name": "verilen yerleştirmeler", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(2)]}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
out, report = replaced(PENCERE, KAPI, [4, 5])
assert report["fields"] == 2
replace.append({"name": "yeni bloğun alanları dolar", "steps": [
    {"op": "execute", "input": {"from": PENCERE, "to": KAPI}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
out, report = replaced(PENCERE, LAMBA, [4, 5])
purged_blocks = [b for b in BLOCKS if b["id"] != PENCERE]
replace.append({"name": "purge: artık yerleşmeyen eski blok silinir, aynı adımda", "steps": [
    {"op": "execute", "input": {"from": PENCERE, "to": LAMBA, "purge": True}, "result": done({**report, "purged": True}),
     "expect": {"entities": entities_expect(out), "blocks": without_ids(purged_blocks)}},
    {"op": "undo", "returns": "Bloğu başkasıyla değiştir", "expect": {"blocks": without_ids(BLOCKS)}},
]})
out, report = replaced(KAPI, LAMBA, [1, 2, 8])
replace.append({"name": "purge: başka bir blok içinde yerleşen eski blok kalır", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": LAMBA, "purge": True}, "result": done(report, [LOCKED_SKIP]),
     "expect": {"blocks": without_ids(BLOCKS)}},
]})
out, report = replaced(KAPI, PENCERE, [1, 2, 8])
replace.append({"name": "plan yazmaz", "steps": [
    {"op": "plan", "input": {"from": KAPI, "to": PENCERE}, "result": done({**report, "revision": "$current"}, [LOCKED_SKIP]), "expect": NOTHING},
]})
replace.append({"name": "bloklar verilmemiş", "steps": [
    {"op": "execute", "input": {"to": PENCERE}, "result": failed("no_block", "Değişecek blok ya da yerine geçecek blok verilmedi. from ve to ile iki bloğun kimliğini verin.", "from"), "expect": NOTHING},
    {"op": "execute", "input": {"from": KAPI}, "result": failed("no_block", "Değişecek blok ya da yerine geçecek blok verilmedi. from ve to ile iki bloğun kimliğini verin.", "to"), "expect": NOTHING},
]})
replace.append({"name": "aynı blok", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": KAPI}, "result": failed("same_block", "Yerleştirmeler aynı bloğa geçirilemez; to ile başka bir blok verin.", "to"), "expect": NOTHING},
]})
for path, inp in (("from", {"from": MISSING, "to": KAPI}), ("to", {"from": KAPI, "to": MISSING})):
    replace.append({"name": f"çizimde olmayan blok ({path})", "steps": [
        {"op": "execute", "input": inp, "result": failed("unknown_block", f"“{MISSING}” kimlikli blok çizimde tanımlı değil. Çizimde tanımlı bir bloğun kimliğini verin.", path), "expect": NOTHING},
    ]})
replace.append({"name": "verilen nesne bloğun yerleştirmesi değil", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(7)]},
     "result": failed("not_insert", "Verilen nesne “Kapı” bloğunun bir yerleştirmesi değil. Bu bloğun yerleştirmelerini verin.", "uids[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(4)]},
     "result": failed("not_insert", "Verilen nesne “Kapı” bloğunun bir yerleştirmesi değil. Bu bloğun yerleştirmelerini verin.", "uids[0]"), "expect": NOTHING},
]})
replace.append({"name": "kimlik olmayan metin", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": ["kapı"]},
     "result": failed("invalid_uid", "$any", "uids[0]"), "expect": NOTHING},
]})
BROKEN = copy.deepcopy(SETUP)
BROKEN["blocks"][2]["attributes"] = [att("ETİKET", P(0, 0), field="No: {NUMARA")]
replace.append({"name": "yeni bloğun alanı derlenmiyorsa hiçbir şey yazılmaz (yalnız bir dosya getirebilir)", "setup": BROKEN, "steps": [
    {"op": "execute", "input": {"from": PENCERE, "to": LAMBA}, "result": failed("invalid_field", "$any", "to"), "expect": NOTHING},
]})
replace.append({"name": "boş liste, olmayan nesne", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": []},
     "result": failed("no_entities", "Değişecek yerleştirme verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"), "expect": NOTHING},
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [NIL]},
     "result": failed("entity_not_found", f"“{NIL}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"), "expect": NOTHING},
]})
replace.append({"name": "yalnız kilitli katmandaki yerleştirme", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(3)]},
     "result": failed("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"), "expect": NOTHING},
]})
replace.append({"name": "beklenen sürüm", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "expectedRevision": "7"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision", "revision": "$current"}}, "expect": NOTHING},
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "expectedRevision": "yedi"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": NOTHING},
]})
out, report = replaced(KAPI, PENCERE, [1, 2, 8])
replace.append({"name": "doğru sürümle yazılır, ikinci kez değiştirecek yerleştirme kalmaz", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "expectedRevision": "$current"}, "result": done(report, [LOCKED_SKIP])},
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(3)]},
     "result": failed("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]")},
]})
out, report = replaced(PENCERE, KAPI, [4])
replace.append({"name": "değeri düşen yok: kapının etiketleri pencerenin değerini alır", "steps": [
    {"op": "execute", "input": {"from": PENCERE, "to": KAPI, "uids": [uid(4)]}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
out, report = replaced(KAPI, LAMBA, [2])
assert report["dropped"] == ["NUMARA", "KAT"]
replace.append({"name": "özniteliksiz bloğa geçen yerleştirmenin değerleri düşer, başka verisi kalır", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": LAMBA, "uids": [uid(2)]}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
out, report = replaced(KAPI, PENCERE, [1])
replace.append({"name": "taşınmış öznitelik yeni blokta taşınabilir değilse yeri düşer", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(1)]}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
out, report = replaced(GRUP, LAMBA, [6])
replace.append({"name": "özniteliksiz iki blok: yalnız blok değişir", "steps": [
    {"op": "execute", "input": {"from": GRUP, "to": LAMBA}, "result": done(report), "expect": {"entities": entities_expect(out)}},
]})
replace.append({"name": "aynı kimlik iki kez verilirse bir kez sayılır", "steps": [
    {"op": "execute", "input": {"from": KAPI, "to": PENCERE, "uids": [uid(2), uid(2)]}, "result": done(replaced(KAPI, PENCERE, [2])[1])},
]})


# ── cad.blocks.query ─────────────────────────────────────────────────────

query = []
BY_NAME = {b["id"]: b for b in BLOCKS}


def counts(block_ids=None, layers=None):
    asked = [b for b in BLOCKS if block_ids is None or b["id"] in block_ids]

    def within(block_id, depth=1):
        out = {}
        for e in BY_NAME[block_id]["entities"]:
            if e["kind"] == "insert":
                out[e["block"]] = out.get(e["block"], 0) + 1
                for k, n in within(e["block"], depth + 1).items():
                    out[k] = out.get(k, 0) + n
        return out

    own, nested = {}, {}
    for e in ENTITIES:
        if e["kind"] != "insert" or (layers is not None and e["layerId"] not in layers):
            continue
        n, by = own.get(e["block"], (0, {}))
        by = dict(by)
        by[e["layerId"]] = by.get(e["layerId"], 0) + 1
        own[e["block"]] = (n + 1, by)
        for k, m in within(e["block"]).items():
            nested[k] = nested.get(k, 0) + m
    return [{"block": b["id"], "name": b["name"], "count": own.get(b["id"], (0, {}))[0],
             "layers": dict(sorted(own.get(b["id"], (0, {}))[1].items())), "nested": nested.get(b["id"], 0)} for b in asked]


def instances(block_ids=None, layers=None, where=()):
    out = []
    for e in ENTITIES:
        if e["kind"] != "insert" or (block_ids is not None and e["block"] not in block_ids):
            continue
        if layers is not None and e["layerId"] not in layers:
            continue
        b = BY_NAME[e["block"]]
        if all(meets(value_of(b, e, f["tag"]), f) for f in where):
            out.append(e)
    return out


LAYER_NAMES = {layer_["id"]: layer_["name"] for layer_ in LAYERS}


def schedule(block_ids=None, layers=None, where=(), columns=None):
    found = instances(block_ids, layers, where)
    asked = [b for b in BLOCKS if block_ids is None or b["id"] in block_ids]
    if columns is None:
        columns = []
        for b in asked:
            for a in b.get("attributes", []):
                if a["tag"] not in columns:
                    columns.append(a["tag"])
    rows = [[BY_NAME[e["block"]]["name"], LAYER_NAMES[e["layerId"]], e["p"]["x"], e["p"]["y"], e["rotation"] * 180.0 / math.pi]
            + [value_of(BY_NAME[e["block"]], e, t) for t in columns] for e in found]
    return {"uids": [uid(e["id"]) for e in found], "columns": ["Blok", "Katman", "X", "Y", "Dönüş"] + columns, "rows": rows}


def answer(**output):
    return done({k: v for k, v in output.items() if v})


def ask(name, inp, result):
    query.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": NOTHING}]})


c = counts()
assert next(x for x in c if x["block"] == KAPI)["nested"] == 2
ask("say: her blok, katmana göre ve başka blokların içinden", {"report": "count"}, answer(counts=c))
ask("say: yalnız bir katmandakiler", {"report": "count", "layers": ["kapilar"]}, answer(counts=counts(layers=["kapilar"])))
ask("say: yerleştirmesi olmayan blok sıfırla", {"report": "count", "blocks": [LAMBA]}, answer(counts=counts([LAMBA])))
ask("say: istenen bloklar çizimin sırasıyla", {"report": "count", "blocks": [GRUP, KAPI]}, answer(counts=counts([KAPI, GRUP])))
for name, where in [
    ("bul: içeren, Türkçe büyük küçük harfe bakmadan", [{"tag": "NUMARA", "op": "contains", "value": "k-"}]),
    ("bul: eşit", [{"tag": "NUMARA", "op": "eq", "value": "k-1"}]),
    ("bul: eşit değil", [{"tag": "NUMARA", "op": "ne", "value": "K-1"}]),
    ("bul: boş değer (verilmemiş, varsayılanı da yok)", [{"tag": "ETİKET", "op": "empty"}]),
    ("bul: boş olmayan", [{"tag": "ETİKET", "op": "notEmpty"}]),
    ("bul: sabit özniteliğin değeri tanımındakidir", [{"tag": "ÜRETİCİ", "op": "eq", "value": "KENTOS"}]),
    ("bul: büyüktür, varsayılan değerle", [{"tag": "GENİŞLİK", "op": "gt", "value": "1.3"}]),
    ("bul: küçüktür", [{"tag": "GENİŞLİK", "op": "lt", "value": "1.3"}]),
    ("bul: bloğunda olmayan etiket boş okunur", [{"tag": "KAT", "op": "empty"}]),
    ("bul: koşulların hepsi", [{"tag": "NUMARA", "op": "contains", "value": "K"}, {"tag": "KAT", "op": "eq", "value": "2"}]),
]:
    ask(name, {"report": "instances", "where": where}, answer(uids=[uid(e["id"]) for e in instances(where=where)]))
ask("bul: blok ve katmanla daraltılır", {"report": "instances", "blocks": [KAPI], "layers": ["kapilar"]},
    answer(uids=[uid(e["id"]) for e in instances([KAPI], ["kapilar"])]))
ask("çizelge: bütün yerleştirmeler, blokların bütün etiketleri", {"report": "schedule"}, answer(**schedule()))
ask("bul: koşulsuz bütün yerleştirmeler, kilitli katmandakiler de", {"report": "instances"}, answer(uids=[uid(e["id"]) for e in instances()]))
ask("çizelge: istenen sütunlar ve bloklar", {"report": "schedule", "blocks": [PENCERE], "columns": ["GENİŞLİK"]},
    answer(**schedule([PENCERE], columns=["GENİŞLİK"])))
ask("çizelge: koşulla", {"report": "schedule", "where": [{"tag": "NUMARA", "op": "contains", "value": "p-"}]},
    answer(**schedule(where=[{"tag": "NUMARA", "op": "contains", "value": "p-"}])))
query.append({"name": "retler: blok, katman, koşul, sütun", "steps": [
    {"op": "execute", "input": {"report": "count", "blocks": [MISSING]},
     "result": failed("unknown_block", f"“{MISSING}” kimlikli blok çizimde tanımlı değil. Çizimde tanımlı bir bloğun kimliğini verin.", "blocks[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"report": "count", "layers": ["yok"]},
     "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layers[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"report": "instances", "where": [{"tag": "NUMARA", "op": "eq"}]},
     "result": failed("invalid_condition", "1. koşul okunamadı: bu karşılaştırma bir değer ister.", "where[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"report": "instances", "where": [{"tag": "KAT", "op": "eq", "value": "1"}, {"tag": "KAT", "op": "lt", "value": "bir"}]},
     "result": failed("invalid_condition", "2. koşul okunamadı: küçüktür ve büyüktür bir sayı ister.", "where[1]"), "expect": NOTHING},
    {"op": "execute", "input": {"report": "instances", "where": [{"tag": " ", "op": "empty"}]},
     "result": failed("invalid_condition", "1. koşul okunamadı: koşulun etiketi boş.", "where[0]"), "expect": NOTHING},
    {"op": "execute", "input": {"report": "schedule", "blocks": [PENCERE], "columns": ["KAT"]},
     "result": failed("unknown_tag", "“KAT” etiketli öznitelik istenen bloklarda yok.", "columns[0]"), "expect": NOTHING},
]})


# ── Writing ──────────────────────────────────────────────────────────────


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def text_of(command, title, note, cases):
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', f'  "command": "{command}",', '  "commandVersion": 1,',
           f'  "title": {compact(title)},', f'  "note": {compact(note)},', f'  "setup": {compact(SETUP)},', '  "cases": [']
    blocks = []
    for case in cases:
        lines = ["    {", f'      "name": {compact(case["name"])},']
        if "setup" in case:
            lines.append(f'      "setup": {compact(case["setup"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(step)}" for step in case["steps"]))
        lines += ["      ]", "    }"]
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out += ["  ]", "}"]
    return "\n".join(out) + "\n"


NOTE = "Written by scripts/fixtures/block_attribute_cases.py from docs/adr/0317; do not edit by hand."
FILES = {
    "cad.blocks.edit.attributes.json": text_of("cad.blocks.edit", "Blok düzenle: öznitelikleri eşitle, alanlar, konum, kart, seçeneklerin kuralları", NOTE, edit),
    "cad.blocks.replace.json": text_of("cad.blocks.replace", "Bloğu başkasıyla değiştir", NOTE, replace),
    "cad.blocks.query.json": text_of("cad.blocks.query", "Blokları say, bul ve çizelgele", NOTE, query),
}


def main():
    for name, cases in (("edit", edit), ("replace", replace), ("query", query)):
        assert len(cases) >= 20, (name, len(cases))
    if "--check" in sys.argv[1:]:
        for name, text in FILES.items():
            if (OUT / name).read_text(encoding="utf-8") != text:
                sys.exit(f"fixtures/commands/v1/{name} güncel değil")
        print(f"{len(edit)} + {len(replace)} + {len(query)} durum tutarlı")
        return
    for name, text in FILES.items():
        (OUT / name).write_text(text, encoding="utf-8")
    print(f"{len(edit)} + {len(replace)} + {len(query)} durum yazıldı")


main()
