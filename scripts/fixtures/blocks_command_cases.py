"""The shared cases of the block commands (docs/adr/0144 §4): cad.blocks.define
(Blok oluştur) and cad.blocks.edit (the Bloklar panel).

    python3 scripts/fixtures/blocks_command_cases.py           # writes the files
    python3 scripts/fixtures/blocks_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.blocks.define.json and cad.blocks.edit.json.
The checks, their order, codes, paths and messages are written here by hand
from the contract (crates/shared/contracts/src/cad_blocks.rs) and the block
rules (crates/shared/contracts/src/blocks.rs), and so is what a definition
becomes (`copies`, `block` below: the contract's rule, not an implementation's
output). The block rules' names are folded here the Turkish way on their own
(I → ı, İ → i), and nesting is followed here with its own depth-first walk.

`--check` rebuilds the files in memory and compare them with the ones on disk.
"""
import json
import math
import sys

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


def piece(kind, i, **fields):
    return {"kind": kind, "id": i, "layerId": "0", "attrs": {}, **fields}


def insert_of(i, layer_id, block, p, scale=1, rotation=0, attrs=None, mirror=False):
    e = {"kind": "insert", "id": i, "layerId": layer_id, "attrs": attrs or {}, "block": block, "p": p, "scale": scale, "rotation": rotation}
    if mirror:
        e["mirror"] = True
    return e


ROGAR = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001"
LAMBA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2002"
DIREK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2003"
ESKI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2004"
ZINCIR_A = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2005"
ZINCIR_B = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2006"
NIL = "00000000-0000-0000-0000-000000000000"
MISSING_BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2fff"

BLOCKS = [
    {"id": ROGAR, "name": "Rögar", "base": P(0, 0), "entities": [
        piece("circle", 1, c=P(0, 0), r=0.75),
        piece("text", 2, p=P(-0.16, -0.18), text="R", height=0.5, rotation=0),
    ], "description": "Kanalizasyon rögarı"},
    {"id": LAMBA, "name": "Lamba", "base": P(0, 0), "entities": [
        piece("circle", 1, c=P(0, 0), r=0.35),
    ]},
    {"id": DIREK, "name": "Aydınlatma direği", "base": P(0, 0), "entities": [
        piece("polygon", 1, pts=[P(-0.3, -0.3), P(0.3, -0.3), P(0.3, 0.3), P(-0.3, 0.3)]),
        piece("line", 2, a=P(0.3, 0), b=P(2.45, 0)),
        {**piece("insert", 3), "block": LAMBA, "p": P(2.8, 0), "scale": 1, "rotation": 0},
    ]},
    {"id": ESKI, "name": "Eski işaret", "base": P(0, 0), "entities": [piece("point", 1, p=P(0, 0))]},
    {"id": ZINCIR_A, "name": "Zincir A", "base": P(0, 0), "entities": [
        {**piece("insert", 1), "block": ZINCIR_B, "p": P(1, 0), "scale": 2, "rotation": 0},
    ]},
    {"id": ZINCIR_B, "name": "Zincir B", "base": P(0, 0), "entities": [piece("line", 1, a=P(0, 0), b=P(1, 0))]},
]
BY_BLOCK = {b["id"]: b for b in BLOCKS}

QUARTER = math.pi / 2
ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "color": "#E5484D", "attrs": {"Tür": "Duvar"}, "label": "D1", "a": P(487000, 4420000), "b": P(487010, 4420000)},
    {"kind": "circle", "id": 2, "layerId": "yapi", "attrs": {}, "lineWeight": 0.5, "c": P(487020, 4420000), "r": 2},
    {"kind": "text", "id": 3, "layerId": "yapi", "attrs": {}, "p": P(487030, 4420000), "text": "A", "height": 1, "rotation": 0},
    insert_of(4, "altyapi", ROGAR, P(487040, 4420000), attrs={"NO": "R-1"}),
    insert_of(5, "altyapi", DIREK, P(487050, 4420000), rotation=QUARTER),
    {"kind": "line", "id": 6, "layerId": "kilitli", "attrs": {}, "a": P(487000, 4420010), "b": P(487010, 4420010)},
    {"kind": "polygon", "id": 7, "layerId": "ada", "attrs": {"Ada": "101"}, "pts": [P(487000, 4420020), P(487010, 4420020), P(487010, 4420030), P(487000, 4420030)]},
    {"kind": "line", "id": 8, "layerId": "gizli", "attrs": {}, "a": P(487000, 4420040), "b": P(487010, 4420040)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1

LAYERS = [
    layer("yapi", "Yapı"),
    layer("altyapi", "Altyapı"),
    layer("kilitli", "Kilitli katman", locked=True),
    layer("gizli", "Gizli katman", visible=False),
    group("plan", "Plan", [layer("ada", "Ada")]),
    group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
]

SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"}


def setup(blocks=BLOCKS, entities=ENTITIES):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ürün komutu",
        "settings": SETTINGS,
        "origin": {"x": 487000, "y": 4420000},
        "layers": LAYERS,
        "activeLayer": "yapi",
        "entities": entities,
        "styles": {"items": [], "categories": []},
        "blocks": blocks,
    }


SETUP = setup()

# ── The rules, written here on their own (docs/adr/0144) ────────────────

WHITE_SPACE = set("\t\n\x0b\x0c\r \u0085      　") | {chr(c) for c in range(0x2000, 0x200B)}
MAX_DEPTH = 16


def fold(name):
    """A name as names are compared: Turkish I (I → ı, İ → i), then each character lowercased."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def name_ok(name):
    return any(c not in WHITE_SPACE for c in name)


def depth_of(block_id, blocks, seen=()):
    """How deep a definition nests (1 without inserts); None past a cycle."""
    by_id = {b["id"]: b for b in blocks}
    if block_id in seen:
        return None
    deepest = 0
    for e in by_id[block_id]["entities"]:
        if e["kind"] == "insert":
            d = depth_of(e["block"], blocks, seen + (block_id,))
            if d is None:
                return None
            deepest = max(deepest, d)
    return deepest + 1


# ── What a definition becomes (the contract) ───────────────────────────


def E(i):
    return json.loads(json.dumps(BY_ID[i]))


def B(block_id):
    return json.loads(json.dumps(BY_BLOCK[block_id]))


def copies(*slots):
    """The objects as a definition holds them: every field kept, local ids 1, 2, … in order."""
    out = []
    for k, slot in enumerate(slots):
        e = E(slot)
        e["id"] = k + 1
        out.append(e)
    return out


def without_id(block):
    b = json.loads(json.dumps(block))
    b.pop("id")
    return b


def definition(name, base, slots, description=None):
    """A new definition without its id, as the `blocks` expectation shows it."""
    d = {"name": name, "base": base, "entities": copies(*slots)}
    if description is not None:
        d["description"] = description
    return d


def blocks_after(*changes, drop=()):
    """The setup's definitions without their ids, with `changes` (id → definition without id) in their places, and new ones appended."""
    out = []
    for b in BLOCKS:
        if b["id"] in drop:
            continue
        out.append(changes_of(changes).get(b["id"], without_id(b)))
    out.extend(v for k, v in changes if k is None)
    return out


def changes_of(changes):
    return {k: v for k, v in changes if k is not None}


SETUP_BLOCKS = [without_id(b) for b in BLOCKS]


def placed(slot, layer_id, block, base):
    """The insert `replace` puts in the objects' place: scale 1, no turn."""
    return {"kind": "insert", "id": slot, "layerId": layer_id, "attrs": {}, "block": block, "p": base, "scale": 1, "rotation": 0}


def uid(i):
    return f"$uidOf:{i}"


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "blocks": SETUP_BLOCKS}
EMPTY_NAME = "Blok adı boş olamaz; bir ad yazın."
NO_ENTITIES = "Bloğa girecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
NO_LAYER = "Yerleştirmenin katmanı verilmedi. Seçilenleri blokla değiştirmek için bir katman verin (Blok oluştur etkin katmanı verir)."
UID_MESSAGE = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
MISSING_MESSAGE = f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
BASE = P(487000, 4420000)


def duplicate(first):
    return failed("duplicate_block", f"Çizimde “{first}” adında bir blok var; başka bir ad verin.", "name")


def not_finite_base(axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return failed("not_finite", f"Taban noktasının {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"base.{axis}")


def locked_object(name, at):
    return failed("layer_locked", f"“{name}” katmanı kilitli; üzerindeki nesne bloğa alınır ama yerine yerleştirme konamaz. Kilidi Katmanlar panelinden açın ya da nesneleri yerinde bırakın.", f"uids[{at}]")


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def unknown_block(block_id):
    return failed("unknown_block", f"“{block_id}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin.", "block")


# ── cad.blocks.define ──────────────────────────────────────────────────

define = []


def defined(name, removed=(), insert_slot=None):
    out = {"block": f"$blockOf:{name}", "removed": list(removed), "revision": "$current"}
    if insert_slot is not None:
        out["insert"] = uid(insert_slot)
        out["id"] = insert_slot
    return {"status": "completed", "output": out, "warnings": []}


define.append({
    "name": "Blok tanımla: nesneler olduğu gibi kopyalanır (katman, renk, kalınlık, öznitelik, etiket; kimlikler 1'den), çizim değişmez; tek adım, geri alınır, aynı kimlikle yinelenir",
    "steps": [
        {"op": "execute", "input": {"name": "Parsel köşesi", "base": BASE, "uids": ["$uidOf:1", "$uidOf:2"]}, "result": defined("Parsel köşesi"),
         "expect": {"ids": IDS, "entities": {"1": E(1), "2": E(2)}, "blocks": SETUP_BLOCKS + [definition("Parsel köşesi", BASE, [1, 2])], "blockIds": {"Parsel köşesi": "new"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "captureBlock", "name": "Parsel köşesi", "as": "kose"},
        {"op": "undo", "returns": "Blok tanımla", "expect": {"blocks": SETUP_BLOCKS, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Blok tanımla", "expect": {"blocks": SETUP_BLOCKS + [definition("Parsel köşesi", BASE, [1, 2])], "blockIds": {"Parsel köşesi": "$block:kose"}}},
    ],
})

# A text that writes an object's label (docs/adr/0175 §4; here the line's, by a persistent id of its own): a
# definition's objects have no persistent ids, so its copy in the definition is a text of its own.
LINKED_TO = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4003"
LINKED = {"kind": "text", "id": 9, "layerId": "yapi", "attrs": {}, "p": P(487005, 4420002), "text": "D1", "height": 1, "rotation": 0,
          "align": "middleCenter", "labelOf": LINKED_TO, "labelScale": 1000}
linked_copy = {k: v for k, v in LINKED.items() if k not in ("labelOf", "labelScale")}
define.append({
    "name": "bağlı yazı bloğa kendi başına bir yazı olarak girer: tanımın nesnelerinin kalıcı kimliği yoktur (ADR 0175 §4)",
    "setup": setup(entities=ENTITIES + [LINKED]),
    "steps": [
        {"op": "execute", "input": {"name": "Etiketli duvar", "base": BASE, "uids": ["$uidOf:1", "$uidOf:9"]}, "result": defined("Etiketli duvar"),
         "expect": {"entities": {"9": LINKED}, "blocks": SETUP_BLOCKS + [{"name": "Etiketli duvar", "base": BASE, "entities": [{**E(1), "id": 1}, {**linked_copy, "id": 2}]}], "revision": "changed"}},
    ],
})

define.append({
    "name": "açıklamasıyla; nesneler girdinin sırasıyla numaralanır",
    "steps": [
        {"op": "execute", "input": {"name": "Yazılı daire", "base": P(487020, 4420000), "uids": ["$uidOf:3", "$uidOf:2"], "description": "Merkezinde A yazan daire"}, "result": defined("Yazılı daire"),
         "expect": {"blocks": SETUP_BLOCKS + [definition("Yazılı daire", P(487020, 4420000), [3, 2], "Merkezinde A yazan daire")], "revision": "changed"}},
    ],
})

define.append({
    "name": "iki kez verilen kimlik tek nesnedir",
    "steps": [
        {"op": "execute", "input": {"name": "Tek", "base": BASE, "uids": ["$uidOf:1", "$uidOf:1", "$uidOf:3"]}, "result": defined("Tek"),
         "expect": {"blocks": SETUP_BLOCKS + [definition("Tek", BASE, [1, 3])], "revision": "changed"}},
    ],
})

define.append({
    "name": "bir yerleştirme de bloğa girer: iç içe blok (direk ve onun lambası, bir düzey daha)",
    "steps": [
        {"op": "execute", "input": {"name": "Direk grubu", "base": P(487050, 4420000), "uids": ["$uidOf:5", "$uidOf:4"]}, "result": defined("Direk grubu"),
         "expect": {"blocks": SETUP_BLOCKS + [definition("Direk grubu", P(487050, 4420000), [5, 4])], "revision": "changed"}},
    ],
})

define.append({
    "name": "kilitli katmandaki nesne de bloğa girer: yerinde kalır",
    "steps": [
        {"op": "execute", "input": {"name": "Kilitli çizgi", "base": BASE, "uids": ["$uidOf:6"]}, "result": defined("Kilitli çizgi"),
         "expect": {"ids": IDS, "blocks": SETUP_BLOCKS + [definition("Kilitli çizgi", BASE, [6])], "revision": "changed"}},
    ],
})

define.append({
    "name": "Seçilenleri blokla değiştir: nesneler silinir, yerine taban noktasına, verilen katmana yeni bloğun yerleştirmesi (ölçek 1, dönüş yok); hepsi tek adım",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "duvar"},
        {"op": "captureUid", "id": 3, "as": "yazi"},
        {"op": "execute", "input": {"name": "Duvar ve yazı", "base": BASE, "uids": ["$uid:duvar", "$uid:yazi"], "replace": True, "layerId": "yapi"}, "result": defined("Duvar ve yazı", ["$uid:duvar", "$uid:yazi"], NEXT),
         "expect": {"ids": [2, 4, 5, 6, 7, 8, NEXT], "entities": {str(NEXT): placed(NEXT, "yapi", "$blockOf:Duvar ve yazı", BASE)}, "uids": {str(NEXT): "new"}, "blocks": SETUP_BLOCKS + [definition("Duvar ve yazı", BASE, [1, 3])], "blockIds": {"Duvar ve yazı": "new"}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Blok tanımla", "note": "Nesneler yerlerine ve kimlikleriyle döner, tanım ve yerleştirme gider.", "expect": {"ids": IDS, "entities": {"1": E(1), "3": E(3)}, "uids": {"1": "duvar", "3": "yazi"}, "blocks": SETUP_BLOCKS, "canUndo": False}},
        {"op": "redo", "returns": "Blok tanımla", "expect": {"ids": [2, 4, 5, 6, 7, 8, NEXT], "blocks": SETUP_BLOCKS + [definition("Duvar ve yazı", BASE, [1, 3])]}},
    ],
})

define.append({
    "name": "yerleştirme gizli katmana uyarıyla konur",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "duvar"},
        {"op": "execute", "input": {"name": "Gizli duvar", "base": BASE, "uids": ["$uid:duvar"], "replace": True, "layerId": "gizli"},
         "result": {"status": "completed", "output": {"block": "$blockOf:Gizli duvar", "insert": uid(NEXT), "id": NEXT, "removed": ["$uid:duvar"], "revision": "$current"},
                    "warnings": [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]},
         "expect": {"ids": [2, 3, 4, 5, 6, 7, 8, NEXT], "entities": {str(NEXT): placed(NEXT, "gizli", "$blockOf:Gizli duvar", BASE)}, "revision": "changed"}},
    ],
})

define.append({
    "name": "değiştirilecek nesnelerden biri kilitli katmandaysa hiçbir şey yazılmaz: kilitli nesne silinemez",
    "steps": [
        {"op": "execute", "input": {"name": "Karışık", "base": BASE, "uids": ["$uidOf:1", "$uidOf:6"], "replace": True, "layerId": "yapi"}, "result": locked_object("Kilitli katman", 1), "expect": NOTHING},
    ],
})

define.append({
    "name": "yerleştirmenin katmanı: çizimde yok, grup, kilitli katman, kilitli grubun katmanı",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "yok"},
         "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "plan"},
         "result": failed("not_a_layer", "“Plan” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "kilitli"},
         "result": failed("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "eski"},
         "result": failed("layer_locked", "“Eski” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"), "expect": NOTHING},
    ],
})

define.append({
    "name": "katman denetimi nesnelerinkinden önce gelir; ikisi de replace olmadan yoktur",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:6"], "replace": True, "layerId": "yok"},
         "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:6"], "replace": False, "layerId": "yok"}, "result": defined("K"),
         "expect": {"ids": IDS, "blocks": SETUP_BLOCKS + [definition("K", BASE, [6])], "revision": "changed"}},
    ],
})

define.append({
    "name": "boş ad: boş metin ve yalnız boşluk (Unicode White_Space, U+0085 dahil)",
    "steps": [
        {"op": "execute", "input": {"name": "", "base": BASE, "uids": ["$uidOf:1"]}, "result": failed("empty_name", EMPTY_NAME, "name"), "expect": NOTHING},
        {"op": "execute", "input": {"name": " \u0085　", "base": BASE, "uids": ["$uidOf:1"]}, "result": failed("empty_name", EMPTY_NAME, "name"), "expect": NOTHING},
    ],
})

define.append({
    "name": "U+FEFF boşluk değildir: yalnız ondan oluşan ad kabul edilir",
    "steps": [
        {"op": "execute", "input": {"name": "﻿", "base": BASE, "uids": ["$uidOf:1"]}, "result": defined("﻿"),
         "expect": {"blocks": SETUP_BLOCKS + [definition("﻿", BASE, [1])], "revision": "changed"}},
    ],
})

define.append({
    "name": "nesne yok; boş ad ondan önce gelir",
    "steps": [
        {"op": "execute", "input": {"name": "Boş", "base": BASE, "uids": []}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "", "base": BASE, "uids": []}, "result": failed("empty_name", EMPTY_NAME, "name"), "expect": NOTHING},
    ],
})

define.append({
    "name": "geçersiz kimlik yazımı, sırayla",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1", "12"]}, "result": failed("invalid_uid", UID_MESSAGE.format("12"), "uids[1]"), "expect": NOTHING},
    ],
})

define.append({
    "name": "sonlu olmayan taban noktası: doğu önce",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"]}, "nonFinite": {"base.y": "NaN"}, "result": not_finite_base("y"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"]}, "nonFinite": {"base.x": "Infinity", "base.y": "NaN"}, "result": not_finite_base("x"), "expect": NOTHING},
    ],
})

define.append({
    "name": "replace katmansız: no_layer; sürümden önce",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "expectedRevision": "x"}, "result": failed("no_layer", NO_LAYER, "layerId"), "expect": NOTHING},
    ],
})

define.append({
    "name": "sürüm: yazımı geçersiz, sonra çizim sürümü değişmiş (conflict)",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1"], "expectedRevision": "01"},
         "result": failed("invalid_revision", "Beklenen sürüm “01” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
        {"op": "captureRevision", "as": "once"},
        {"op": "execute", "input": {"name": "İlk", "base": BASE, "uids": ["$uidOf:1"]}, "result": defined("İlk"), "expect": {"revision": "changed"}},
        {"op": "execute", "input": {"name": "İkinci", "base": BASE, "uids": ["$uidOf:2"], "expectedRevision": "$once"}, "result": conflict(),
         "expect": {"blocks": SETUP_BLOCKS + [definition("İlk", BASE, [1])], "revision": "same"}},
    ],
})

define.append({
    "name": "çizimde olmayan nesne",
    "steps": [
        {"op": "execute", "input": {"name": "K", "base": BASE, "uids": ["$uidOf:1", MISSING]}, "result": failed("entity_not_found", MISSING_MESSAGE, "uids[1]"), "expect": NOTHING},
    ],
})

define.append({
    "name": "çizimde aynı adda blok var: Türkçe büyük küçük harfle (RÖGAR, AYDINLATMA DİREĞİ); ileti ilk tanımın adını verir",
    "steps": [
        {"op": "execute", "input": {"name": "RÖGAR", "base": BASE, "uids": ["$uidOf:1"]}, "result": duplicate("Rögar"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "AYDINLATMA DİREĞİ", "base": BASE, "uids": ["$uidOf:1"]}, "result": duplicate("Aydınlatma direği"), "expect": NOTHING},
        {"op": "execute", "input": {"name": "Zincir a", "base": BASE, "uids": ["$uidOf:1"]}, "result": duplicate("Zincir A"), "expect": NOTHING},
    ],
})

define.append({
    "name": "İ ile I ayrı harflerdir: “ZINCIR B” başka bir addır",
    "steps": [
        {"op": "execute", "input": {"name": "ZINCIR B", "base": BASE, "uids": ["$uidOf:1"]}, "result": defined("ZINCIR B"),
         "expect": {"blocks": SETUP_BLOCKS + [definition("ZINCIR B", BASE, [1])], "revision": "changed"}},
    ],
})

define.append({
    "name": "plan: tanım (kimliği boş UUID) ve replace'te yerleştirme (yuvası 0) ile silinecekler; hiçbir şey yazılmaz",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "daire"},
        {"op": "plan", "input": {"name": "Plan", "base": BASE, "uids": ["$uid:daire"], "replace": True, "layerId": "altyapi"},
         "result": {"status": "completed", "output": {"block": {"id": NIL, "name": "Plan", "base": BASE, "entities": copies(2)}, "insert": placed(0, "altyapi", NIL, BASE), "removed": ["$uid:daire"], "revision": "$current"}, "warnings": []},
         "expect": NOTHING},
        {"op": "plan", "input": {"name": "Plan", "base": BASE, "uids": ["$uid:daire"], "description": "d"},
         "result": {"status": "completed", "output": {"block": {"id": NIL, "name": "Plan", "base": BASE, "entities": copies(2), "description": "d"}, "removed": [], "revision": "$current"}, "warnings": []},
         "expect": NOTHING},
    ],
})

define.append({
    "name": "validate: yazılsa gelecek uyarıyla, hiçbir şey yazmadan",
    "steps": [
        {"op": "validate", "input": {"name": "V", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "gizli"},
         "result": {"status": "completed", "output": None, "warnings": [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]}, "expect": NOTHING},
        {"op": "validate", "input": {"name": "Rögar", "base": BASE, "uids": ["$uidOf:1"]}, "result": duplicate("Rögar"), "expect": NOTHING},
    ],
})

# Sixteen levels: D1 holds D2 … D16 holds a line; a block holding an insert of D1 is seventeen.
DEEP = []
for k in range(1, MAX_DEPTH + 1):
    bid = f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d30{k:02d}"
    if k < MAX_DEPTH:
        inner = [{**piece("insert", 1), "block": f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d30{k + 1:02d}", "p": P(0, 0), "scale": 1, "rotation": 0}]
    else:
        inner = [piece("line", 1, a=P(0, 0), b=P(1, 0))]
    DEEP.append({"id": bid, "name": f"D{k}", "base": P(0, 0), "entities": inner})
DEEP_ENTITIES = [insert_of(1, "yapi", DEEP[0]["id"], P(487000, 4420000))]
assert depth_of(DEEP[0]["id"], DEEP) == MAX_DEPTH

define.append({
    "name": "en çok 16 düzey: 16 düzeylik bloğun yerleştirmesinden yapılan blok 17 düzey olur; ileti yeni bloğu adlandırır",
    "setup": setup(DEEP, DEEP_ENTITIES),
    "steps": [
        {"op": "execute", "input": {"name": "Çok derin", "base": BASE, "uids": ["$uidOf:1"]},
         "result": failed("block_too_deep", "Bloklar en çok 16 düzey iç içe olabilir; “Çok derin” bloğu daha derin.", "uids"),
         "expect": {"ids": [1], "canUndo": False, "revision": "same", "blocks": [without_id(b) for b in DEEP]}},
        {"op": "validate", "input": {"name": "Düz", "base": BASE, "uids": ["$uidOf:1"], "replace": True, "layerId": "yapi"},
         "result": failed("block_too_deep", "Bloklar en çok 16 düzey iç içe olabilir; “Düz” bloğu daha derin.", "uids")},
    ],
})

# ── cad.blocks.edit ────────────────────────────────────────────────────

# A table is the drawing's alone (docs/adr/0184 §1): a block holds none.
TABLE = {"kind": "table", "id": 9, "layerId": "yapi", "attrs": {}, "p": P(487000, 4420060), "rotation": 0, "height": 1.25, "rows": [2.5, 2.5],
         "columns": [8, 10], "cells": [["Ad", "Alan"], ["7", "600.00"]]}
TABLE_IN_BLOCK = "Seçilenlerde tablo var; tablo bloğa konamaz. Tabloyu seçimden çıkarın."
T_NOTHING = {**NOTHING, "ids": IDS + [9]}
define.append({
    "name": "tablo bloğa konamaz: table_in_block, ilk tablonun yeriyle; nesnelerin çizimde olmasından sonra (ADR 0184 §1)",
    "setup": setup(entities=ENTITIES + [TABLE]),
    "steps": [
        {"op": "execute", "input": {"name": "Çizelge", "base": BASE, "uids": ["$uidOf:1", "$uidOf:9"]}, "result": failed("table_in_block", TABLE_IN_BLOCK, "uids[1]"), "expect": T_NOTHING},
        {"op": "execute", "input": {"name": "Çizelge", "base": BASE, "uids": ["$uidOf:9", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]},
         "result": failed("entity_not_found", "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[1]"),
         "note": "Olmayan nesne tablodan önce söylenir.", "expect": T_NOTHING},
    ],
})

edit = []


def edited(changed=(), removed=(), deleted=(), insert_slot=None):
    out = {"changed": list(changed), "removed": list(removed), "deleted": list(deleted), "revision": "$current"}
    if insert_slot is not None:
        out["insert"] = uid(insert_slot)
        out["id"] = insert_slot
    return {"status": "completed", "output": out, "warnings": []}


def renamed(block_id, name):
    b = without_id(B(block_id))
    b["name"] = name
    return b


def rebased(block_id, base):
    b = without_id(B(block_id))
    b["base"] = base
    return b


def redefined(block_id, slots, base=None):
    b = without_id(B(block_id))
    b["entities"] = copies(*slots)
    if base is not None:
        b["base"] = base
    return b


edit.append({
    "name": "Yeniden adlandır: bloğun adı değişir, kimliği ve yerleştirmeleri kalır; adımı “Blok değiştir”, geri alınır",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "Kanalizasyon rögarı"}, "result": edited([ROGAR]),
         "expect": {"ids": IDS, "entities": {"4": E(4)}, "blocks": blocks_after((ROGAR, renamed(ROGAR, "Kanalizasyon rögarı"))), "blockIds": {"Kanalizasyon rögarı": ROGAR}, "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": SETUP_BLOCKS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Blok değiştir", "expect": {"blocks": blocks_after((ROGAR, renamed(ROGAR, "Kanalizasyon rögarı")))}},
    ],
})

edit.append({
    "name": "aynı ad bir şey değiştirmez: hiçbir şey yazılmaz; yalnız büyük küçük harfi değişen ad bir değişikliktir",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "Rögar"}, "result": edited(), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "RÖGAR"}, "result": edited([ROGAR]),
         "expect": {"blocks": blocks_after((ROGAR, renamed(ROGAR, "RÖGAR"))), "revision": "changed"}},
    ],
})

edit.append({
    "name": "başka bir bloğun adı verilemez (Türkçe harf katlamasıyla)",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "lamba"}, "result": duplicate("lamba"), "expect": NOTHING,
         "note": "Kurallar tanımları sırayla okur: adı değişen Rögar ilk sıradadır, Lamba onun adını ikinci kez taşır."},
        {"op": "execute", "input": {"operation": "rename", "block": ZINCIR_B, "name": "ZİNCİR A"}, "result": duplicate("Zincir A"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "boş ad; ad verilmemiş",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "  "}, "result": failed("empty_name", EMPTY_NAME, "name"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR}, "result": failed("empty_name", EMPTY_NAME, "name"), "expect": NOTHING},
    ],
})

NO_BLOCK = "Değiştirilecek blok verilmedi. Bloğun kimliğini verin."

edit.append({
    "name": "blok verilmemiş (temizle dışında her işlem ister): önce o denetlenir",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "name": ""}, "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "remove"}, "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "rebase"}, "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "çizimde tanımlı olmayan blok, sürümden sonra",
    "steps": [
        {"op": "execute", "input": {"operation": "rename", "block": MISSING_BLOCK, "name": "Yeni"}, "result": unknown_block(MISSING_BLOCK), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "remove", "block": MISSING_BLOCK, "expectedRevision": "x"},
         "result": failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "Taban noktası: tanımın taban noktası değişir, her yerleştirme farkı kadar kayar; aynı nokta bir şey değiştirmez",
    "steps": [
        {"op": "execute", "input": {"operation": "rebase", "block": ROGAR, "base": P(1, 2)}, "result": edited([ROGAR]),
         "expect": {"blocks": blocks_after((ROGAR, rebased(ROGAR, P(1, 2)))), "entities": {"4": E(4)}, "revision": "changed"}},
        {"op": "execute", "input": {"operation": "rebase", "block": ROGAR, "base": P(1, 2)}, "result": edited(), "expect": {"revision": "same"}},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": SETUP_BLOCKS, "canUndo": False}},
    ],
})

edit.append({
    "name": "taban noktası verilmemiş; sonlu değil",
    "steps": [
        {"op": "execute", "input": {"operation": "rebase", "block": ROGAR}, "result": failed("no_base", "Yeni taban noktası verilmedi. Bloğun taban noktasını verin.", "base"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "rebase", "block": ROGAR, "base": P(0, 0)}, "nonFinite": {"base.y": "-Infinity"}, "result": not_finite_base("y"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "Yeniden tanımla: içindekiler seçilen nesnelerden kurulur (kimlikler 1'den), taban noktası kalır; nesneler çizimde kalır, yerleştirmeler yenisini gösterir",
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:2", "$uidOf:3"]}, "result": edited([ROGAR]),
         "expect": {"ids": IDS, "entities": {"2": E(2), "3": E(3), "4": E(4)}, "blocks": blocks_after((ROGAR, redefined(ROGAR, [2, 3]))), "revision": "changed"}},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": SETUP_BLOCKS}},
    ],
})

edit.append({
    "name": "yeniden tanımlarken taban noktası da verilebilir",
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": LAMBA, "uids": ["$uidOf:2"], "base": P(487020, 4420000)}, "result": edited([LAMBA]),
         "expect": {"blocks": blocks_after((LAMBA, redefined(LAMBA, [2], P(487020, 4420000)))), "revision": "changed"}},
    ],
})

edit.append({
    "name": "yeniden tanımlanan blok da tablo alamaz: table_in_block (ADR 0184 §1)",
    "setup": setup(entities=ENTITIES + [TABLE]),
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:9"]}, "result": failed("table_in_block", TABLE_IN_BLOCK, "uids[0]"), "expect": T_NOTHING},
    ],
})

edit.append({
    "name": "yeniden tanımla ve nesneleri blokla değiştir: nesneler silinir, yerine bu bloğun yerleştirmesi taban noktasına; tek adım",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "daire"},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uid:daire"], "base": P(487020, 4420000), "replace": True, "layerId": "altyapi"},
         "result": edited([ROGAR], deleted=["$uid:daire"], insert_slot=NEXT),
         "expect": {"ids": [1, 3, 4, 5, 6, 7, 8, NEXT], "entities": {str(NEXT): placed(NEXT, "altyapi", ROGAR, P(487020, 4420000))}, "uids": {str(NEXT): "new"},
                    "blocks": blocks_after((ROGAR, redefined(ROGAR, [2], P(487020, 4420000)))), "revision": "changed"}},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"ids": IDS, "entities": {"2": E(2)}, "uids": {"2": "daire"}, "blocks": SETUP_BLOCKS, "canUndo": False}},
    ],
})

edit.append({
    "name": "tanım aynı kalsa da replace nesneleri yerleştirmeyle değiştirir: changed boş",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "daire"},
        {"op": "execute", "input": {"operation": "redefine", "block": LAMBA, "uids": ["$uid:daire"], "base": P(487020, 4420000)}, "result": edited([LAMBA]), "expect": {"revision": "changed"}},
        {"op": "execute", "input": {"operation": "redefine", "block": LAMBA, "uids": ["$uid:daire"], "base": P(487020, 4420000), "replace": True, "layerId": "yapi"},
         "result": edited([], deleted=["$uid:daire"], insert_slot=NEXT),
         "expect": {"ids": [1, 3, 4, 5, 6, 7, 8, NEXT], "entities": {str(NEXT): placed(NEXT, "yapi", LAMBA, P(487020, 4420000))}, "revision": "changed"}},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"ids": IDS, "blocks": blocks_after((LAMBA, redefined(LAMBA, [2], P(487020, 4420000))))}},
    ],
})

edit.append({
    "name": "blok kendini içeremez: doğrudan (Rögar'ın yerleştirmesiyle) ya da başka bloklar yoluyla (Lamba'ya direk: direk Lamba'yı içerir)",
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:4"]},
         "result": failed("block_cycle", "“Rögar” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla).", "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": LAMBA, "uids": ["$uidOf:5"]},
         "result": failed("block_cycle", "“Lamba” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla).", "uids"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "yeniden tanımlamanın girdisi: nesne yok, geçersiz kimlik, sonlu olmayan taban, replace katmansız; çizimde olmayan nesne",
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": []}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["kose"]}, "result": failed("invalid_uid", UID_MESSAGE.format("kose"), "uids[0]"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:2"], "base": P(0, 0)}, "nonFinite": {"base.x": "NaN"}, "result": not_finite_base("x"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:2"], "replace": True}, "result": failed("no_layer", NO_LAYER, "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": [MISSING]}, "result": failed("entity_not_found", MISSING_MESSAGE, "uids[0]"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "yeniden tanımlayıp değiştirirken kilitli katmandaki nesne silinemez",
    "steps": [
        {"op": "execute", "input": {"operation": "redefine", "block": ROGAR, "uids": ["$uidOf:2", "$uidOf:6"], "replace": True, "layerId": "yapi"}, "result": locked_object("Kilitli katman", 1), "expect": NOTHING},
    ],
})

edit.append({
    "name": "Blok sil: kullanılmayan tanım silinir; geri alınınca yerine döner",
    "steps": [
        {"op": "execute", "input": {"operation": "remove", "block": ESKI}, "result": edited(removed=[ESKI]),
         "expect": {"blocks": blocks_after(drop=[ESKI]), "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Blok sil", "expect": {"blocks": SETUP_BLOCKS, "blockIds": {"Eski işaret": ESKI}}},
    ],
})

edit.append({
    "name": "çizimde yerleştirmesi olan blok silinmez",
    "steps": [
        {"op": "execute", "input": {"operation": "remove", "block": ROGAR}, "result": failed("block_in_use", "“Rögar” bloğu çizimde 1 kez yerleştirilmiş; silinemez. Önce yerleştirmelerini silin.", "block"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "başka bir bloğun içinde kullanılan blok silinmez; kullanan blok kullanılmasa da",
    "steps": [
        {"op": "execute", "input": {"operation": "remove", "block": LAMBA}, "result": failed("block_in_use", "“Lamba” bloğu “Aydınlatma direği” bloğunun içinde kullanılıyor; silinemez.", "block"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "remove", "block": ZINCIR_B}, "result": failed("block_in_use", "“Zincir B” bloğu “Zincir A” bloğunun içinde kullanılıyor; silinemez.", "block"), "expect": NOTHING},
    ],
})

edit.append({
    "name": "Blokları temizle: kullanılmayan her tanım gider, yalnız onların kullandıkları da (Zincir A, sonra Zincir B); çıktı çizimin sırasıyla; tek adım",
    "steps": [
        {"op": "execute", "input": {"operation": "purge"}, "result": edited(removed=[ESKI, ZINCIR_A, ZINCIR_B]),
         "expect": {"blocks": blocks_after(drop=[ESKI, ZINCIR_A, ZINCIR_B]), "canUndo": True, "revision": "changed"}},
        {"op": "execute", "input": {"operation": "purge"}, "result": edited(), "note": "Kullanılmayan kalmadı: hiçbir şey yazılmaz.", "expect": {"revision": "same"}},
        {"op": "undo", "returns": "Blokları temizle", "expect": {"blocks": SETUP_BLOCKS, "canUndo": False}},
    ],
})

edit.append({
    "name": "plan: değişecek tanım kimliğiyle, silinecekler; hiçbir şey yazılmaz",
    "steps": [
        {"op": "plan", "input": {"operation": "rename", "block": LAMBA, "name": "Fener"},
         "result": {"status": "completed", "output": {"changed": [{**B(LAMBA), "name": "Fener"}], "removed": [], "deleted": [], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "plan", "input": {"operation": "purge"},
         "result": {"status": "completed", "output": {"changed": [], "removed": [ESKI, ZINCIR_A, ZINCIR_B], "deleted": [], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "captureUid", "id": 3, "as": "yazi"},
        {"op": "plan", "input": {"operation": "redefine", "block": LAMBA, "uids": ["$uid:yazi"], "replace": True, "layerId": "yapi"},
         "result": {"status": "completed", "output": {"changed": [{**B(LAMBA), "entities": copies(3)}], "removed": [], "insert": placed(0, "yapi", LAMBA, P(0, 0)), "deleted": ["$uid:yazi"], "revision": "$current"}, "warnings": []},
         "expect": NOTHING},
    ],
})

edit.append({
    "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
    "steps": [
        {"op": "captureRevision", "as": "once"},
        {"op": "execute", "input": {"operation": "remove", "block": ESKI}, "result": edited(removed=[ESKI]), "expect": {"revision": "changed"}},
        {"op": "execute", "input": {"operation": "rename", "block": ROGAR, "name": "R", "expectedRevision": "$once"}, "result": conflict(),
         "expect": {"blocks": blocks_after(drop=[ESKI]), "revision": "same"}},
    ],
})

edit.append({
    "name": "validate: hiçbir şey yazmadan",
    "steps": [
        {"op": "validate", "input": {"operation": "remove", "block": ESKI}, "result": {"status": "completed", "output": None, "warnings": []}, "expect": NOTHING},
        {"op": "validate", "input": {"operation": "remove", "block": ROGAR}, "result": failed("block_in_use", "“Rögar” bloğu çizimde 1 kez yerleştirilmiş; silinemez. Önce yerleştirmelerini silin.", "block"), "expect": NOTHING},
    ],
})

# ── attributes (docs/adr/0144 §7) ─────────────────────────────────────


def attribute(tag, p, height, rotation=0, prompt=None, value=None):
    """An attribute definition as the contract writes it: the optional prompt and default only when given."""
    a = {"tag": tag}
    if prompt is not None:
        a["prompt"] = prompt
    if value is not None:
        a["value"] = value
    a.update({"p": p, "height": height, "rotation": rotation})
    return a


NO = attribute("NO", P(0.9, 0.15), 0.5, prompt="Rögar numarası", value="R-?")
KOT = attribute("KOT", P(0.9, -0.55), 0.4, prompt="Kapak kotu")
NO_ATTRIBUTES = "Öznitelik listesi verilmedi. Bloğun bütün özniteliklerini verin; boş liste hepsini kaldırır."
EMPTY_TAG = "Öznitelik etiketi boş olamaz; her özniteliğe bir etiket verin."


def with_attributes(block_id, attributes):
    """The definition without its id, its attribute list `attributes` (none written when empty)."""
    b = without_id(B(block_id))
    b.pop("attributes", None)
    if attributes:
        b["attributes"] = attributes
    return b


def duplicate_tag(tag, at):
    return failed("duplicate_tag", f"“{tag}” etiketi listede iki kez var; her etiket bir kez olmalı.", f"attributes[{at}].tag")


def not_finite_place(tag, axis, at):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return failed("not_finite", f"“{tag}” özniteliğinin yerinin {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"attributes[{at}].p.{axis}")


def not_finite_turn(tag, at):
    return failed("not_finite", f"“{tag}” özniteliğinin açısı sonlu bir sayı değil (NaN ya da sonsuz). Açıyı sonlu bir sayıyla verin.", f"attributes[{at}].rotation")


def invalid_height(tag, at):
    return failed("invalid_height", f"“{tag}” özniteliğinin yazı yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre, bloğun kendi ölçüsünde).", f"attributes[{at}].height")


def attributes_input(block_id, attributes):
    return {"operation": "attributes", "block": block_id, "attributes": attributes}


edit.append({
    "name": "Öznitelikler: tanımın öznitelik listesi bütün yazılır, yerleştirmeler onu gösterir; adımı “Blok değiştir”, geri alınır",
    "steps": [
        {"op": "execute", "input": attributes_input(ROGAR, [NO, KOT]), "result": edited([ROGAR]),
         "expect": {"ids": IDS, "entities": {"4": E(4)}, "blocks": blocks_after((ROGAR, with_attributes(ROGAR, [NO, KOT]))), "canUndo": True, "dirty": True, "revision": "changed"},
         "note": "Yerleştirmenin kendisi değişmez: değerleri kendi özniteliklerindedir, tanım yalnız yazılarını tanımlar."},
        {"op": "undo", "returns": "Blok değiştir", "expect": {"blocks": SETUP_BLOCKS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Blok değiştir", "expect": {"blocks": blocks_after((ROGAR, with_attributes(ROGAR, [NO, KOT])))}},
    ],
})

edit.append({
    "name": "aynı liste bir şey değiştirmez; sıra bir değişikliktir; boş liste öznitelikleri kaldırır, özniteliksiz tanımda bir şey değiştirmez",
    "steps": [
        {"op": "execute", "input": attributes_input(ROGAR, []), "result": edited(), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, KOT]), "result": edited([ROGAR]), "expect": {"revision": "changed"}},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, KOT]), "result": edited(),
         "expect": {"blocks": blocks_after((ROGAR, with_attributes(ROGAR, [NO, KOT]))), "revision": "same"}},
        {"op": "execute", "input": attributes_input(ROGAR, [KOT, NO]), "result": edited([ROGAR]),
         "expect": {"blocks": blocks_after((ROGAR, with_attributes(ROGAR, [KOT, NO]))), "revision": "changed"}},
        {"op": "execute", "input": attributes_input(ROGAR, []), "result": edited([ROGAR]), "expect": {"blocks": SETUP_BLOCKS, "revision": "changed"}},
    ],
})

edit.append({
    "name": "liste verilmemiş; boş ya da yalnız boşluk etiket; listede ikinci kez aynı etiket (tam eşitlik: büyük küçük harf ayrı etikettir)",
    "steps": [
        {"op": "execute", "input": {"operation": "attributes", "block": ROGAR}, "result": failed("no_attributes", NO_ATTRIBUTES, "attributes"), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [attribute("", P(0, 0), 0.5)]), "result": failed("empty_tag", EMPTY_TAG, "attributes[0].tag"), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, attribute(" 　", P(0, 0), 0.5)]), "result": failed("empty_tag", EMPTY_TAG, "attributes[1].tag"), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, KOT, attribute("NO", P(0, 1), 0.3)]), "result": duplicate_tag("NO", 2), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, attribute("no", P(0, 1), 0.3)]), "result": edited([ROGAR]),
         "expect": {"blocks": blocks_after((ROGAR, with_attributes(ROGAR, [NO, attribute("no", P(0, 1), 0.3)]))), "revision": "changed"}},
    ],
})

edit.append({
    "name": "yer ve açı sonlu (önce doğu), yükseklik sıfırdan büyük ve sonlu; öznitelikler sırayla denetlenir",
    "steps": [
        {"op": "execute", "input": attributes_input(ROGAR, [NO]), "nonFinite": {"attributes[0].p.y": "NaN"}, "result": not_finite_place("NO", "y", 0), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO]), "nonFinite": {"attributes[0].p.x": "Infinity", "attributes[0].rotation": "NaN"}, "result": not_finite_place("NO", "x", 0), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO]), "nonFinite": {"attributes[0].rotation": "-Infinity"}, "result": not_finite_turn("NO", 0), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO]), "nonFinite": {"attributes[0].height": "Infinity"}, "result": invalid_height("NO", 0), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [attribute("NO", P(0, 0), 0)]), "result": invalid_height("NO", 0), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [NO, attribute("KOT", P(0, 0), -0.4)]), "result": invalid_height("KOT", 1), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(ROGAR, [attribute("A", P(0, 0), 0), attribute("", P(0, 0), 0.5)]), "result": invalid_height("A", 0), "expect": NOTHING,
         "note": "Birinci özniteliğin yüksekliği ikincinin etiketinden önce denetlenir."},
    ],
})

edit.append({
    "name": "öznitelikler: denetim sırası; liste bloktan sonra, sürüm ve çizimden önce",
    "steps": [
        {"op": "execute", "input": {"operation": "attributes", "attributes": [NO]}, "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(MISSING_BLOCK, [attribute(" ", P(0, 0), 0.5)]), "result": failed("empty_tag", EMPTY_TAG, "attributes[0].tag"), "expect": NOTHING},
        {"op": "execute", "input": attributes_input(MISSING_BLOCK, [NO]), "result": unknown_block(MISSING_BLOCK), "expect": NOTHING},
        {"op": "plan", "input": attributes_input(LAMBA, [NO]),
         "result": {"status": "completed", "output": {"changed": [{**B(LAMBA), "attributes": [NO]}], "removed": [], "deleted": [], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
    ],
})

# White space other than the plain space, escaped so a reader sees it.
INVISIBLE = "\u0085                 　﻿"


def compact(v):
    text = json.dumps(v, ensure_ascii=False, separators=(", ", ": "))
    return "".join(f"\\u{ord(c):04x}" if c in INVISIBLE else c for c in text)


def write(command, title, note, cases):
    fixture = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note, "setup": SETUP, "cases": cases}
    out = ["{"]
    for key in ["format", "version", "command", "commandVersion", "title", "note"]:
        out.append(f'  "{key}": {compact(fixture[key])},')
    s = fixture["setup"]
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])},')
    out.append('    "blocks": [')
    out.append(",\n".join(f"      {compact(b)}" for b in s["blocks"]))
    out.append("    ]")
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


# Checks of this file's own rules against its data, so a case cannot contradict them.
assert all(name_ok(b["name"]) for b in BLOCKS)
assert len({fold(b["name"]) for b in BLOCKS}) == len(BLOCKS)
assert fold("AYDINLATMA DİREĞİ") == fold("Aydınlatma direği") and fold("ZINCIR B") != fold("Zincir B")
assert depth_of(DIREK, BLOCKS) == 2 and depth_of(ZINCIR_A, BLOCKS) == 2
assert not name_ok(" \u0085　") and name_ok("﻿")

write(
    "cad.blocks.define",
    "Blok tanımla: doğrulama, plan, yazma, geri alma",
    "ADR 0144 §4. Denetim sırası: adın boş olmaması (yalnız boşluk da boştur, Unicode White_Space; U+FEFF boşluk değildir); en az bir kimlik (no_entities), her kimliğin yazımı; taban noktasının sonlu olması (doğu önce); replace ile katman verilmesi (no_layer); beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması (iki kez verilen tek nesnedir); tablonun olmaması (table_in_block, ilk tablonun uids[i]'si; ADR 0184 §1); blok kuralları: adın çizimde bir kez olması (Türkçe büyük küçük harf katlamasıyla: I → ı, İ → i; duplicate_block, ileti ilk tanımın adını verir), en çok 16 düzey (block_too_deep); replace ile yerleştirmenin katmanı (var, grup değil, kilitli değil; gizliyse layer_hidden uyarısı), sonra nesnelerin katmanı kilitli değil (layer_locked, uids[i]). Nesneler olduğu gibi kopyalanır: geometrisi, katmanı, rengi, kalınlığı, öznitelikleri, etiketi, kimlikleri girdinin sırasıyla 1'den. replace ile nesneler silinir, yerine yeni bloğun yerleştirmesi taban noktasına, verilen katmana konur (ölçek 1, dönüş yok, öznitelik yok). Adım “Blok tanımla”. Plan tanımı boş UUID kimlikle, yerleştirmeyi yuva 0 ve boş UUID blokla verir. Kurulumdaki en büyük nesne kimliği 8. $blockOf:Ad çizimdeki o adlı bloğun kimliğidir.",
    define,
)
write(
    "cad.blocks.edit",
    "Blok tanımını değiştir: yeniden adlandır, yeniden tanımla, taban noktası, sil, temizle, öznitelikler",
    "ADR 0144 §4. Denetim sırası: blok verilmesi (temizle dışında; no_block); işlemin girdisi: rename adın boş olmaması (empty_name), redefine en az bir kimlik ve yazımları, verildiyse sonlu taban noktası, replace ile katman (no_layer), rebase taban noktası (no_base) ve sonluluğu; beklenen sürümün yazımı, sonra çizimin sürümü; bloğun çizimde olması (unknown_block); işlemin kendisi: rename ve rebase blok kuralları (duplicate_block), redefine nesnelerin çizimde olması, tablonun olmaması (table_in_block), blok kuralları (block_cycle: blok kendini doğrudan ya da başka bloklar yoluyla içeremez; block_too_deep), replace ile katman denetimleri; remove bloğun çizimde ya da başka bir tanımda yerleştirmesi olmaması (block_in_use). attributes (ADR 0144 §7) girdide listeyi ister (no_attributes; boş liste öznitelikleri kaldırır), sonra her özniteliği sırayla: etiketin boş ya da yalnız boşluk olmaması (empty_tag), listede önceki bir etiketin tam aynısı olmaması (duplicate_tag; büyük küçük harf ayrı etikettir), yerin sonlu olması (önce doğu), açının sonlu olması (not_finite), yüksekliğin sıfırdan büyük ve sonlu olması (invalid_height); bu denetimler blok verilmesinden sonra, sürümden ve bloğun çizimde olmasından önce yapılır. Bir şey değiştirmeyen istek hiçbir şey yazmaz; sonuç tamamlanır, changed ve removed boş. Temizle kullanılmayan tanımları, yalnız onların kullandıklarıyla birlikte siler; removed çizimin sırasıyladır. Adımlar: rename, redefine, rebase, attributes “Blok değiştir”; remove “Blok sil”; purge “Blokları temizle”. Kurulumdaki en büyük nesne kimliği 8.",
    edit,
)
print(f"{len(define)} + {len(edit)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
