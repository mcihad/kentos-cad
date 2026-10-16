#!/usr/bin/env python3
"""The shared cases of `cad.blocks.insert` (docs/adr/0315 §4; plan/cad.md CAD-BLOCK-03, 04).

    python3 scripts/fixtures/block_insert_cases.py           # writes the file
    python3 scripts/fixtures/block_insert_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.blocks.insert.json with Python's standard library, no KentOS code. The rules are
written here from the ADR and the contract (crates/shared/contracts/src/cad_blocks.rs):

- the block by `block` (its id), else by `name`, names compared with Turkish case folding (I → ı, İ → i);
- the factors: `scale` (absent 1), `yScale` (absent `scale`), each times the unit's factor; a `yScale` equal to the
  x factor is not written (one way to write an insert);
- the unit's factor: a unitless block takes the drawing's unit in metres (a millimetre drawing 1/1000, as the
  ratio of whole numbers divides); any other block 1, and one drawn in another unit than the drawing's is said
  (`insert_units_differ`);
- attribute values by tag; a tag the block does not have is refused;
- the checks in order: a block named, `at` finite (east first), the turn finite, `scale` then `yScale` positive and
  finite, the expected revision, the block known, the tags, the layer.
"""
import json
import math
import sys

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


def group(i, name, children):
    return {"id": i, "name": name, "type": "group", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": children}


def P(x, y):
    return {"x": x, "y": y}


def piece(kind, i, **fields):
    return {"kind": kind, "id": i, "layerId": "", "attrs": {}, **fields}


KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3101"
KUZEY = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3102"
PENCERE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3103"
MISSING_BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3fff"

BLOCKS = [
    # A door drawn in millimetres: its geometry is metres, as every drawing's (ADR 0315 §4).
    {"id": KAPI, "name": "Kapı", "base": P(0, 0), "units": "mm", "entities": [
        piece("line", 1, a=P(0, 0), b=P(0.9, 0)),
        piece("arc", 2, c=P(0, 0), r=0.9, a0=0, a1=math.pi / 2),
    ], "attributes": [
        {"tag": "NO", "prompt": "Kapı numarası", "value": "K-?", "p": P(0.2, 0.2), "height": 0.15, "rotation": 0},
        {"tag": "TÜR", "p": P(0.2, 0.4), "height": 0.15, "rotation": 0},
    ]},
    # A north arrow without a unit: its numbers are the drawing's units.
    {"id": KUZEY, "name": "Kuzey oku", "base": P(0, 0), "units": "unitless", "entities": [
        piece("polygon", 1, pts=[P(0, 0), P(0.5, -0.5), P(0, 2), P(-0.5, -0.5)]),
    ]},
    # A window of an unknown unit: real size, said nothing.
    {"id": PENCERE, "name": "Pencere", "base": P(0, 0), "entities": [
        piece("polygon", 1, pts=[P(0, 0), P(1.2, 0), P(1.2, 0.2), P(0, 0.2)]),
        piece("circle", 2, c=P(0.6, 0.1), r=0.05),
    ]},
]
BY_NAME = {b["id"]: b["name"] for b in BLOCKS}

ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "duvar", "attrs": {}, "a": P(0, 0), "b": P(10, 0)},
]
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1

LAYERS = [
    layer("duvar", "Duvar"),
    layer("kapilar", "Kapılar"),
    layer("kilitli", "Kilitli katman", locked=True),
    layer("gizli", "Gizli katman", visible=False),
    group("plan", "Plan", [layer("mobilya", "Mobilya")]),
]

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "drawingFont": "barlow"}
MM = {**SETTINGS, "drawingUnit": "mm"}


def setup(settings=SETTINGS):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Blok ekle",
        "settings": settings,
        "origin": {"x": 0, "y": 0},
        "layers": LAYERS,
        "activeLayer": "duvar",
        "entities": ENTITIES,
        "styles": {"items": [], "categories": []},
        "blocks": BLOCKS,
    }


SETUP = setup()


def fold(name):
    """A name as names are compared: Turkish I (I → ı, İ → i), then each character lowercased."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def unit_factor(block, drawing):
    """A unitless block takes the drawing's unit: its metres as the ratio of whole numbers divides (1 ÷ 1000)."""
    if block.get("units") != "unitless":
        return 1
    return {"m": 1, "cm": 1 / 100, "mm": 1 / 1000}[drawing]


def insert(slot, block_id, at, layer_id="kapilar", scale=None, y_scale=None, rotation=None, mirror=False,
           attrs=None, color=None, drawing="m"):
    """The insert as the contract writes it."""
    block = next(b for b in BLOCKS if b["id"] == block_id)
    f = unit_factor(block, drawing)
    sx = (1 if scale is None else scale) * f
    e = {"kind": "insert", "id": slot, "layerId": layer_id, "attrs": attrs or {}, "block": block_id, "p": at,
         "scale": sx, "rotation": 0 if rotation is None else rotation}
    if y_scale is not None and y_scale * f != sx:
        e["yScale"] = y_scale * f
    if mirror:
        e["mirror"] = True
    if color is not None:
        e["color"] = color
    return e


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def done(slot, block_id, factor=1, warnings=()):
    return {"status": "completed",
            "output": {"uid": "$uid", "id": slot, "block": block_id, "unitFactor": factor, "revision": "$current"},
            "warnings": list(warnings)}


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
NO_BLOCK = "Yerleştirilecek blok verilmedi. Bloğu kimliğiyle (block) ya da adıyla (name) verin."
CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
            "yeniden hazırlayın.")
HIDDEN = {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.",
          "path": "layerId"}


def units_differ(name, unit, drawing):
    return {"code": "insert_units_differ",
            "message": f"“{name}” bloğu {unit} biriminde çizilmiş, çizim {drawing}; blok gerçek boyutuyla yerleşti.",
            "path": "block"}


def not_finite_at(axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return failed("not_finite", f"Yerleştirme noktasının {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). "
                                "Koordinatı sonlu bir sayıyla verin.", f"at.{axis}")


def invalid_scale(what, path):
    return failed("invalid_scale", f"{what} sıfırdan büyük, sonlu bir sayı olmalı. Pozitif bir ölçek verin.", path)


AT = P(3, 0)
cases = []


def add(name, steps, setup_doc=None, note=None):
    c = {"name": name, "steps": steps}
    if setup_doc is not None:
        c["setup"] = setup_doc
    if note is not None:
        c["note"] = note
    cases.append(c)


def written(slot, entity, extra=None):
    out = {"ids": IDS + [slot], "entities": {str(slot): entity}, "canUndo": True, "canRedo": False, "dirty": True,
           "revision": "changed"}
    out.update(extra or {})
    return out


# ── Writes ───────────────────────────────────────────────────────────────

e = insert(NEXT, PENCERE, AT)
add("kimliğiyle yerleşir, tek adımda geri alınır, aynı kalıcı kimlikle yinelenir", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar"}, "result": done(NEXT, PENCERE),
     "expect": written(NEXT, e)},
    {"op": "captureUid", "id": NEXT, "as": "konan"},
    {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
    {"op": "redo", "returns": "Ekle", "expect": {"ids": IDS + [NEXT], "entities": {str(NEXT): e},
                                                 "uids": {str(NEXT): "konan"}, "canUndo": True}},
])
assert fold("PENCERE") == fold("Pencere")
add("adıyla bulunur: Türkçe büyük küçük harf katlamasıyla (PENCERE, Pencere)", [
    {"op": "execute", "input": {"name": "PENCERE", "at": AT, "layerId": "kapilar"}, "result": done(NEXT, PENCERE),
     "expect": written(NEXT, insert(NEXT, PENCERE, AT))},
])
assert fold("KUZEY OKU") == "kuzey oku" and fold("KAPI") == "kapı" and fold("KAPİ") != "kapı"
add("I ı olur: KAPI kapının adıdır, KAPİ değildir", [
    {"op": "execute", "input": {"name": "KAPI", "at": AT, "layerId": "kapilar"},
     "result": done(NEXT, KAPI, warnings=[units_differ("Kapı", "mm", "m")]),
     "expect": written(NEXT, insert(NEXT, KAPI, AT))},
    {"op": "execute", "input": {"name": "KAPİ", "at": AT, "layerId": "kapilar"},
     "result": failed("unknown_block", "“KAPİ” adlı blok çizimde tanımlı değil. Bloklar panelindeki bir bloğun adını "
                                       "ya da kimliğini verin.", "name"),
     "expect": {"ids": IDS + [NEXT], "revision": "same"}},
])
add("kimlik verilince ad okunmaz", [
    {"op": "execute", "input": {"block": PENCERE, "name": "Kapı", "at": AT, "layerId": "kapilar"},
     "result": done(NEXT, PENCERE), "expect": written(NEXT, insert(NEXT, PENCERE, AT))},
])
add("eşit olmayan ölçek: x 1,5, y 1; yScale yazılır", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "scale": 1.5, "yScale": 1, "layerId": "kapilar"},
     "result": done(NEXT, PENCERE),
     "expect": written(NEXT, insert(NEXT, PENCERE, AT, scale=1.5, y_scale=1))},
])
add("yalnız yScale: x 1, y 2", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "yScale": 2, "layerId": "kapilar"},
     "result": done(NEXT, PENCERE),
     "expect": written(NEXT, insert(NEXT, PENCERE, AT, y_scale=2))},
])
add("x'e eşit yScale yazılmaz: bir yerleştirmenin tek yazılışı", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "scale": 2, "yScale": 2, "layerId": "kapilar"},
     "result": done(NEXT, PENCERE), "expect": written(NEXT, insert(NEXT, PENCERE, AT, scale=2))},
])
add("dönüş radyanla, aynalama, kendi rengi", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "rotation": math.pi / 2, "mirror": True,
                                "color": "#E5484D", "layerId": "kapilar"},
     "result": done(NEXT, PENCERE),
     "expect": written(NEXT, insert(NEXT, PENCERE, AT, rotation=math.pi / 2, mirror=True, color="#E5484D"))},
])
add("öznitelik değerleri etiketleriyle; verilmeyen varsayılanını gösterir (yazılmaz)", [
    {"op": "execute", "input": {"block": KAPI, "at": AT, "layerId": "kapilar", "attributes": {"NO": "K-12"}},
     "result": done(NEXT, KAPI, warnings=[units_differ("Kapı", "mm", "m")]),
     "expect": written(NEXT, insert(NEXT, KAPI, AT, attrs={"NO": "K-12"}))},
])
add("etiketler tam aynısıyla: “no” bir etiket değil", [
    {"op": "execute", "input": {"block": KAPI, "at": AT, "layerId": "kapilar", "attributes": {"NO": "K-1", "no": "x"}},
     "result": failed("unknown_tag", "“Kapı” bloğunda “no” etiketli öznitelik yok. Bloğun etiketlerinden birini "
                                     "verin.", "attributes.no"),
     "expect": NOTHING},
])

# ── Units (ADR 0315 §4) ──────────────────────────────────────────────────

add("birimsiz blok metre çizimde: çarpan 1", [
    {"op": "execute", "input": {"block": KUZEY, "at": AT, "layerId": "kapilar"}, "result": done(NEXT, KUZEY, 1),
     "expect": written(NEXT, insert(NEXT, KUZEY, AT))},
])
assert unit_factor(BLOCKS[1], "mm") == 1 / 1000
add("birimsiz blok milimetre çizimde: 1'i 1 mm, ölçek 1/1000 ile çarpılır", [
    {"op": "execute", "input": {"block": KUZEY, "at": AT, "scale": 2, "yScale": 3, "layerId": "kapilar"},
     "result": done(NEXT, KUZEY, 1 / 1000),
     "expect": written(NEXT, insert(NEXT, KUZEY, AT, scale=2, y_scale=3, drawing="mm"))},
], setup_doc=setup(MM))
add("milimetrede çizilmiş blok metre çizimde gerçek boyutuyla; söylenir", [
    {"op": "execute", "input": {"block": KAPI, "at": AT, "layerId": "kapilar"},
     "result": done(NEXT, KAPI, warnings=[units_differ("Kapı", "mm", "m")]),
     "expect": written(NEXT, insert(NEXT, KAPI, AT))},
])
add("milimetrede çizilmiş blok milimetre çizimde: söylenecek bir şey yok", [
    {"op": "execute", "input": {"block": KAPI, "at": AT, "layerId": "kapilar"}, "result": done(NEXT, KAPI),
     "expect": written(NEXT, insert(NEXT, KAPI, AT, drawing="mm"))},
], setup_doc=setup(MM))
add("birimi bilinmeyen blok gerçek boyutuyla, söylenmeden", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar"}, "result": done(NEXT, PENCERE),
     "expect": written(NEXT, insert(NEXT, PENCERE, AT, drawing="mm"))},
], setup_doc=setup(MM))

# ── Plan and validate ────────────────────────────────────────────────────

add("plan yerleştirmeyi yuva 0 ile ve çarpanı verir; hiçbir şey yazmaz", [
    {"op": "plan", "input": {"block": KUZEY, "at": AT, "scale": 2, "layerId": "kapilar"},
     "result": {"status": "completed", "output": {"insert": insert(0, KUZEY, AT, scale=2, drawing="mm"),
                                                  "unitFactor": 1 / 1000, "revision": "$current"}, "warnings": []},
     "expect": NOTHING},
], setup_doc=setup(MM))
add("validate uyarıları verir, yazmaz", [
    {"op": "validate", "input": {"block": PENCERE, "at": AT, "layerId": "gizli"},
     "result": {"status": "completed", "output": None, "warnings": [HIDDEN]}, "expect": NOTHING},
])

# ── Refusals, in the contract's order ────────────────────────────────────

add("blok verilmedi; boş ad da verilmemiştir", [
    {"op": "execute", "input": {"at": AT, "layerId": "kapilar"}, "result": failed("no_block", NO_BLOCK, "block"),
     "expect": NOTHING},
    {"op": "execute", "input": {"name": " \t", "at": AT, "layerId": "kapilar"},
     "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
])
add("blok verilmeden önce nokta denetlenmez: no_block NaN'dan önce", [
    {"op": "execute", "input": {"at": AT, "layerId": "kapilar"}, "nonFinite": {"at.x": "NaN"},
     "result": failed("no_block", NO_BLOCK, "block"), "expect": NOTHING},
])
add("NaN ya da sonsuz nokta: önce doğu", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar"}, "nonFinite": {"at.x": "NaN"},
     "result": not_finite_at("x"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar"}, "nonFinite": {"at.y": "Infinity"},
     "result": not_finite_at("y"), "expect": NOTHING},
])
add("sonsuz dönüş", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "rotation": 0, "layerId": "kapilar"},
     "nonFinite": {"rotation": "-Infinity"},
     "result": failed("not_finite", "Dönüş sonlu bir sayı değil (NaN ya da sonsuz). Dönüşü radyan olarak sonlu bir "
                                    "sayıyla verin.", "rotation"), "expect": NOTHING},
])
add("ölçek sıfır, y ölçeği eksi: önce x", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "scale": 0, "yScale": -1, "layerId": "kapilar"},
     "result": invalid_scale("Ölçek", "scale"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "yScale": -1, "layerId": "kapilar"},
     "result": invalid_scale("Y ölçeği", "yScale"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "scale": 1, "layerId": "kapilar"},
     "nonFinite": {"scale": "NaN"}, "result": invalid_scale("Ölçek", "scale"), "expect": NOTHING},
])
add("beklenen sürüm: yazımı, sonra çizimin sürümü", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar", "expectedRevision": "01"},
     "result": failed("invalid_revision", "Beklenen sürüm “01” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                                          "yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                      "expectedRevision"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kapilar", "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                "path": "expectedRevision", "revision": "$current"}},
     "expect": NOTHING},
])
add("çizimde olmayan blok, kimliğiyle", [
    {"op": "execute", "input": {"block": MISSING_BLOCK, "at": AT, "layerId": "kapilar"},
     "result": failed("unknown_block", f"“{MISSING_BLOCK}” kimlikli blok çizimde tanımlı değil. Bloklar panelindeki "
                                       "bir bloğun adını ya da kimliğini verin.", "block"), "expect": NOTHING},
])
add("blok bilinmeden etiketler denetlenmez, etiketler katmandan önce", [
    {"op": "execute", "input": {"name": "Yok", "at": AT, "layerId": "yok", "attributes": {"X": "1"}},
     "result": failed("unknown_block", "“Yok” adlı blok çizimde tanımlı değil. Bloklar panelindeki bir bloğun adını "
                                       "ya da kimliğini verin.", "name"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "yok", "attributes": {"X": "1"}},
     "result": failed("unknown_tag", "“Pencere” bloğunda “X” etiketli öznitelik yok. Bloğun etiketlerinden birini "
                                     "verin.", "attributes.X"), "expect": NOTHING},
])
add("katman: yok, grup, kilitli; gizli uyarıyla yazar", [
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "yok"},
     "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "layerId"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "plan"},
     "result": failed("not_a_layer", "“Plan” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                                     "katman seçin.", "layerId"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "kilitli"},
     "result": failed("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da "
                                      "başka bir katmanı etkinleştirin.", "layerId"), "expect": NOTHING},
    {"op": "execute", "input": {"block": PENCERE, "at": AT, "layerId": "gizli"},
     "result": done(NEXT, PENCERE, warnings=[HIDDEN]), "expect": written(NEXT, insert(NEXT, PENCERE, AT, "gizli"))},
])


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.blocks.insert",',
           '  "commandVersion": 1,', '  "title": "Blok ekle: doğrulama, plan, yazma, geri alma",',
           f'  "note": {compact(__doc__.split(chr(10) + chr(10), 2)[2].strip())},',
           f'  "setup": {compact(SETUP)},', '  "cases": [']
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
    path = "fixtures/commands/v1/cad.blocks.insert.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write()
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
