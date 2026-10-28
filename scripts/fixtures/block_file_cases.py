#!/usr/bin/env python3
"""The shared cases of `file.import.block` (docs/adr/0316 §3; plan/cad.md CAD-BLOCK-07).

    python3 scripts/fixtures/block_file_cases.py           # writes the files
    python3 scripts/fixtures/block_file_cases.py --check   # writes nothing; compares

Writes the source drawing fixtures/blocks/v1/ofis-bloklari.kcad (a `.kcad` v1 JSON file, which the command reads as
it reads a v2 one) and fixtures/commands/v1/file.import.block.json, with Python's standard library, no KentOS code.
What a drawing becomes is the independent reference of Çizimler arası alışveriş (scripts/fixtures/exchange_cases.py:
`file_block`, `take`, written from docs/adr/0193); this script adds the command's own rules from ADR 0316 §3:

- `drawing` (the default): the file as one block, named `name`, else the file's name without its extension;
- `blocks`: the file's blocks by `names` (every one when absent); a name the drawing has is kept and said
  (`skipped`), or with `replace` redefined in place (its id stays, every insert shows the file's);
- the refusals in order: a path not absolute, a file not there, a file that is no drawing; the expected revision;
  nothing to make a block of, a name the file has not.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import exchange_cases as x  # noqa: E402  (the exchange's own reference, no KentOS code)

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures" / "blocks" / "v1" / "ofis-bloklari.kcad"
OUT = ROOT / "fixtures" / "commands" / "v1" / "file.import.block.json"
FILE = "$fixture:blocks/v1/ofis-bloklari.kcad"


def P(x_, y):
    return {"x": x_, "y": y}


def st(color, weight=0.25, line="continuous"):
    return {"color": color, "lineType": line, "lineWeight": weight}


def layer(i, name, style, children=None, kind="layer", locked=False):
    return {"id": i, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": style, "children": children or []}


def piece(kind, i, layer_id="", **f):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **f}


KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6101"
PENCERE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6102"
GRUP = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6103"
OUR_KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6201"

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "drawingFont": "barlow"}

THEIRS = {
    "format": "kentos.document", "version": 1, "name": "Ofis blokları", "settings": SETTINGS,
    "origin": P(0, 0),
    "layers": [
        layer("mimari", "Mimari", st("fg"), [
            layer("kapi", "A-KAPI", st("#C8A165")),
            layer("duvar", "A-DUVAR", st("#E5484D", 0.5)),
        ], kind="group"),
        layer("yazi", "Yazı", st("fg", 0.18)),
    ],
    "activeLayer": "kapi",
    "entities": [
        {**piece("insert", 1, "kapi"), "block": GRUP, "p": P(4, 3), "scale": 1, "rotation": 0},
        piece("line", 2, "duvar", a=P(0, 0), b=P(10, 0)),
        piece("line", 3, "duvar", a=P(10, 0), b=P(10, 8)),
        piece("text", 4, "yazi", p=P(2, 6), text="Ofis", height=0.5, rotation=0),
        {**piece("table", 5, "yazi"), "p": P(0, 10), "rotation": 0.0, "height": 0.5, "rows": [1.0], "columns": [4.0],
         "cells": [["Oda"]]},
    ],
    "styles": {"items": [], "categories": []},
    "blocks": [
        {"id": KAPI, "name": "Kapı 90", "base": P(0, 0), "units": "m", "entities": [
            piece("line", 1, a=P(0, 0), b=P(0, 0.9), byLayer=["color", "lineType", "lineWeight", "transparency"]),
            piece("arc", 2, c=P(0, 0), r=0.9, a0=0, a1=math.pi / 2),
        ]},
        {"id": PENCERE, "name": "Pencere", "base": P(0, 0), "entities": [
            piece("polygon", 1, "duvar", pts=[P(0, 0), P(1.2, 0), P(1.2, 0.2), P(0, 0.2)]),
        ]},
        {"id": GRUP, "name": "Kapı grubu", "base": P(0, 0), "entities": [
            {**piece("insert", 1), "block": KAPI, "p": P(0, 0), "scale": 1, "rotation": 0},
            piece("line", 2, "duvar", a=P(-1, 0), b=P(3, 0)),
        ]},
    ],
}

OURS = {
    "format": "kentos.document", "version": 1, "name": "Blok al", "settings": SETTINGS, "origin": P(0, 0),
    "layers": [layer("cizim", "Çizim", st("fg")), layer("kapilar", "Kapılar", st("#C8A165")),
               layer("kilitli", "Kilitli", st("fg"), locked=True)],
    "activeLayer": "cizim",
    "entities": [{**piece("insert", 1, "kapilar"), "block": OUR_KAPI, "p": P(20, 0), "scale": 1, "rotation": 0}],
    "styles": {"items": [], "categories": []},
    "blocks": [{"id": OUR_KAPI, "name": "Kapı 90", "base": P(0, 0), "entities": [
        piece("line", 1, a=P(0, 0), b=P(0, 1.0)),
    ]}],
}
IDS = [1]


def fresh(ours, theirs):
    """ADR 0316 §3: the file's blocks whose ids the drawing has take new ids (`fresh:` here), inserts with them."""
    have = {b["id"] for b in ours.get("blocks", [])}
    theirs = copy.deepcopy(theirs)
    moved = {b["id"]: "fresh:" + b["id"] for b in theirs.get("blocks", []) if b["id"] in have}

    def follow(e):
        if e.get("kind") == "insert" and e["block"] in moved:
            e["block"] = moved[e["block"]]
    for b in theirs.get("blocks", []):
        b["id"] = moved.get(b["id"], b["id"])
        for e in b["entities"]:
            follow(e)
    for e in theirs["entities"]:
        follow(e)
    return theirs


def named(after):
    """A new id the case cannot know, as `$blockOf:` the block's name."""
    names = {b["id"]: b["name"] for b in after["blocks"]}

    def of(v):
        if isinstance(v, str) and v.startswith("fresh:"):
            return "$blockOf:" + names[v]
        if isinstance(v, list):
            return [of(i) for i in v]
        if isinstance(v, dict):
            return {k: of(i) for k, i in v.items()}
        return v
    return of(after)


def without_ids(blocks):
    out = []
    for b in blocks:
        b = copy.deepcopy(b)
        b.pop("id", None)
        out.append(b)
    return out


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same",
           "blocks": without_ids(OURS["blocks"])}


LEFT = [{"code": "objects_left_out", "message": "1 tablo bloğa konamadığı için alınmadı.", "path": "path"}]


def union(a, b):
    if a is None:
        return b
    if b is None:
        return a
    return [min(a[0], b[0]), min(a[1], b[1]), max(a[2], b[2]), max(a[3], b[3])]


def entity_box(by_id, e):
    """An object's box (docs/adr/0726 §2): a line's ends, an arc's ends and the quadrant points it passes, a polygon's
    corners, an insert's block placed at its point (these cases turn none); a text's is left out, and `sized` checks
    that it lies inside."""
    k = e["kind"]
    if k == "line":
        a, b = e["a"], e["b"]
        return [min(a["x"], b["x"]), min(a["y"], b["y"]), max(a["x"], b["x"]), max(a["y"], b["y"])]
    if k == "arc":
        c, r, a0, a1 = e["c"], e["r"], e["a0"], e["a1"]
        pts = [(c["x"] + r * math.cos(a0), c["y"] + r * math.sin(a0)), (c["x"] + r * math.cos(a1), c["y"] + r * math.sin(a1))]
        span = (a1 - a0) % (2 * math.pi) or 2 * math.pi
        for q in range(4):
            if (q * math.pi / 2 - a0) % (2 * math.pi) <= span + 1e-12:
                pts.append((c["x"] + r * [1, 0, -1, 0][q], c["y"] + r * [0, 1, 0, -1][q]))
        return [min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)]
    if k == "polygon":
        xs, ys = [q["x"] for q in e["pts"]], [q["y"] for q in e["pts"]]
        return [min(xs), min(ys), max(xs), max(ys)]
    if k == "insert":
        assert e.get("rotation", 0) == 0 and not e.get("mirror"), "these cases turn no insert"
        inner = block_box(by_id, by_id[e["block"]])
        if inner is None:
            return None
        s, px, py = e["scale"], e["p"]["x"], e["p"]["y"]
        return [px + s * inner[0], py + s * inner[1], px + s * inner[2], py + s * inner[3]]
    if k == "text":
        return None
    raise ValueError(k)


def block_box(by_id, b):
    """A block's box at the origin at scale 1: its pieces about its base point."""
    box = None
    for e in b["entities"]:
        box = union(box, entity_box(by_id, e))
    if box is None:
        return None
    bx, by = b["base"]["x"], b["base"]["y"]
    for e in b["entities"]:
        if e["kind"] == "text":
            assert box[0] < e["p"]["x"] < box[2] and box[1] < e["p"]["y"] < box[3] - e["height"], "a text inside"
    return [box[0] - bx, box[1] - by, box[2] - bx, box[3] - by]


def sized(after, base):
    """The answer's sizes (docs/adr/0726 §2): the blocks `after` has that `base` has not (by id and name) or has
    otherwise, in the drawing's order."""
    by_id = {b["id"]: b for b in after["blocks"]}
    before = {(b["id"], b["name"]): b for b in base["blocks"]}
    out = []
    for b in after["blocks"]:
        if before.get((b["id"], b["name"])) == b:
            continue
        box = block_box(by_id, b)
        if box is None:
            continue
        size = {"block": b["id"] if not b["id"].startswith("fresh:") else "$blockOf:" + b["name"], "name": b["name"]}
        if "units" in b:
            size["units"] = b["units"]
        size["width"] = box[2] - box[0]
        size["depth"] = box[3] - box[1]
        out.append(size)
    return out


def done(block=None, added=(), changed=(), skipped=(), layers=0, after=None, planned=False, base=None):
    out = {"added": list(added), "changed": list(changed), "layers": layers, "revision": "$current"}
    if block is not None:
        out["block"] = block
    if skipped:
        out["skipped"] = list(skipped)
    if after is not None:
        # The new ids the case cannot know: the drawing's `$new`, by name, or anything in a plan.
        sizes = sized(after, base or OURS)
        for size in sizes:
            if size["block"] == "$new":
                size["block"] = "$any" if planned else "$blockOf:" + size["name"]
        if sizes:
            out["sizes"] = sizes
    # A drawing as one block leaves its table out (exchange_cases.file_block's `left`).
    return {"status": "completed", "output": out, "warnings": LEFT if block is not None else []}


def diff(after):
    """What the drawing has after: the new blocks' ids in order, those whose definition changed."""
    ours = {b["id"]: b for b in OURS["blocks"]}
    added = [b["id"] for b in after["blocks"] if b["id"] not in ours]
    changed = [b["id"] for b in after["blocks"] if b["id"] in ours and b != ours[b["id"]]]
    return added, changed


def made_layers(after):
    return len(x.all_ids(after["layers"])) - len(x.all_ids(OURS["layers"]))


cases = []


def add(name, steps, note=None, setup=None):
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    if note:
        c["note"] = note
    cases.append(c)


# ── drawing: the file as one block ───────────────────────────────────────

after, left = x.file_block(OURS, THEIRS, "ofis-bloklari")
assert left == {"images": 0, "tables": 1}, left  # LEFT says this
new = after["blocks"][-1]
nested = [b["id"] for b in after["blocks"][len(OURS["blocks"]):-1]]
add("çizim tek blok olur: adı dosyanın adı, iç blokları ve katmanları yollarıyla; tek adımda geri alınır", [
    {"op": "execute", "input": {"path": FILE},
     "result": done("$blockOf:ofis-bloklari", nested + ["$blockOf:ofis-bloklari"], layers=made_layers(after), after=after),
     "expect": {"blocks": without_ids(after["blocks"]), "canUndo": True, "revision": "changed"}},
    {"op": "undo", "returns": "Dosyadan blok: ofis-bloklari.kcad", "expect": {"blocks": without_ids(OURS["blocks"])}},
])
assert new["name"] == "ofis-bloklari" and any(b["name"] == "Kapı 90 (2)" for b in after["blocks"])
after_named, _ = x.file_block(OURS, THEIRS, "Ofis")
add("çizim tek blok olur: verilen adla", [
    {"op": "execute", "input": {"path": FILE, "name": "Ofis"},
     "result": done("$blockOf:Ofis", nested + ["$blockOf:Ofis"], layers=made_layers(after_named), after=after_named),
     "expect": {"blocks": without_ids(after_named["blocks"])}},
])
after_taken, _ = x.file_block(OURS, THEIRS, "Kapı 90")
add("çizimde olan ad “(2)” alır", [
    {"op": "execute", "input": {"path": FILE, "name": "Kapı 90"},
     "result": done("$blockOf:Kapı 90 (3)", nested + ["$blockOf:Kapı 90 (3)"], layers=made_layers(after_taken),
                    after=after_taken),
     "expect": {"blocks": without_ids(after_taken["blocks"])}},
], note="Dosyanın kendi Kapı 90'ı da gelir ve “Kapı 90 (2)” olur; yeni blok “(3)”.")
add("plan okur, hiçbir şey yazmaz", [
    {"op": "plan", "input": {"path": FILE},
     "result": done("$any", nested + ["$any"], layers=made_layers(after), after=after, planned=True), "expect": NOTHING},
])

# ── blocks: the file's blocks by name ────────────────────────────────────


def taken(names, same):
    return x.take(OURS, THEIRS, {"blocks": names}, same)


after_pencere = taken(["Pencere"], "skip")
add("bloklar adlarıyla: yeni ad eklenir, katmanı yoluyla gelir", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["Pencere"]},
     "result": done(added=[PENCERE], layers=made_layers(after_pencere), after=after_pencere),
     "expect": {"blocks": without_ids(after_pencere["blocks"])}},
])
after_skip = taken(["Kapı 90", "Pencere"], "skip")
add("çizimde olan ad korunur ve söylenir", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["Kapı 90", "Pencere"]},
     "result": done(added=[PENCERE], skipped=["Kapı 90"], layers=made_layers(after_skip), after=after_skip),
     "expect": {"blocks": without_ids(after_skip["blocks"])}},
])
after_replace = taken(["Kapı 90"], "replace")
add("replace: çizimdeki aynı adlı blok dosyanınkiyle yeniden tanımlanır, kimliği kalır; ad büyük harfle de bulunur", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["KAPI 90"], "replace": True},
     "result": done(changed=[OUR_KAPI], layers=made_layers(after_replace), after=after_replace),
     "expect": {"blocks": without_ids(after_replace["blocks"]), "blockIds": {"Kapı 90": OUR_KAPI}}},
])
after_all = taken([b["name"] for b in THEIRS["blocks"]], "skip")
add("adlar verilmezse bütün bloklar, yerleştirdikleriyle", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks"},
     "result": done(added=[b["id"] for b in after_all["blocks"] if b["id"] not in (OUR_KAPI,)],
                    skipped=["Kapı 90"], layers=made_layers(after_all), after=after_all),
     "expect": {"blocks": without_ids(after_all["blocks"])}},
])
after_all_r = taken([b["name"] for b in THEIRS["blocks"]], "replace")
add("bütün bloklar, replace ile", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "replace": True},
     "result": done(added=[b["id"] for b in after_all_r["blocks"] if b["id"] != OUR_KAPI], changed=[OUR_KAPI],
                    layers=made_layers(after_all_r), after=after_all_r),
     "expect": {"blocks": without_ids(after_all_r["blocks"])}},
])

after_lower = taken(["Pencere"], "skip")
add("ad büyük küçük harfe bakmadan bulunur", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["PENCERE"]},
     "result": done(added=diff(after_lower)[0], layers=made_layers(after_lower), after=after_lower),
     "expect": {"blocks": without_ids(after_lower["blocks"])}},
])
after_group = taken(["Kapı grubu"], "skip")
add("iç blok getiren blok: çizimdeki aynı adlı iç blok korunur", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["Kapı grubu"]},
     "result": done(added=diff(after_group)[0], changed=diff(after_group)[1], layers=made_layers(after_group),
                    after=after_group),
     "expect": {"blocks": without_ids(after_group["blocks"])}},
])
after_group_r = taken(["Kapı grubu"], "replace")
add("iç blok getiren blok, replace ile: iç blok da dosyanınki olur", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["Kapı grubu"], "replace": True},
     "result": done(added=diff(after_group_r)[0], changed=diff(after_group_r)[1],
                    layers=made_layers(after_group_r), after=after_group_r),
     "expect": {"blocks": without_ids(after_group_r["blocks"])}},
])
add("bloklar: plan yazmaz", [
    {"op": "plan", "input": {"path": FILE, "mode": "blocks", "names": ["Pencere"]},
     "result": done(added=diff(after_pencere)[0], layers=made_layers(after_pencere), after=after_pencere),
     "expect": NOTHING},
])

after_twice, _ = x.file_block(after, fresh(after, THEIRS), "ofis-bloklari")
after_twice = named(after_twice)
nested_twice = [b["id"] for b in after_twice["blocks"][len(after["blocks"]):-1]]
add("aynı dosya iki kez: ikinci blok “(2)” alır, iç blokları da yeniden gelir", [
    {"op": "execute", "input": {"path": FILE},
     "result": done("$blockOf:ofis-bloklari", nested + ["$blockOf:ofis-bloklari"], layers=made_layers(after),
                    after=after)},
    {"op": "execute", "input": {"path": FILE},
     "result": done("$blockOf:ofis-bloklari (2)", nested_twice + ["$blockOf:ofis-bloklari (2)"], layers=0,
                    after=after_twice, base=after),
     "expect": {"blocks": without_ids(after_twice["blocks"])}},
])
add("boş ad dosyanın adına döner", [
    {"op": "execute", "input": {"path": FILE, "name": "  "},
     "result": done("$blockOf:ofis-bloklari", nested + ["$blockOf:ofis-bloklari"], layers=made_layers(after),
                    after=after),
     "expect": {"blocks": without_ids(after["blocks"])}},
])
add("doğrulama dosyayı okur, hiçbir şey yazmaz", [
    {"op": "validate", "input": {"path": FILE}, "result": {"status": "completed", "output": None, "warnings": []},
     "expect": NOTHING},
])
add("geri alınan alma yinelenir", [
    {"op": "execute", "input": {"path": FILE, "name": "Ofis"},
     "result": done("$blockOf:Ofis", nested + ["$blockOf:Ofis"], layers=made_layers(after_named),
                    after=after_named)},
    {"op": "undo", "returns": "Dosyadan blok: ofis-bloklari.kcad", "expect": {"blocks": without_ids(OURS["blocks"])}},
    {"op": "redo", "returns": "Dosyadan blok: ofis-bloklari.kcad",
     "expect": {"blocks": without_ids(after_named["blocks"]), "canRedo": False}},
])
add("doğru sürümle yazılır", [
    {"op": "execute", "input": {"path": FILE, "name": "Ofis", "expectedRevision": "$current"},
     "result": done("$blockOf:Ofis", nested + ["$blockOf:Ofis"], layers=made_layers(after_named),
                    after=after_named)},
])

# ── Refusals ─────────────────────────────────────────────────────────────

add("göreli yol", [
    {"op": "execute", "input": {"path": "ofis.kcad"},
     "result": failed("invalid_file_path", "“ofis.kcad” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).", "path"),
     "expect": NOTHING},
])
add("olmayan dosya", [
    {"op": "execute", "input": {"path": "$fixture:blocks/v1/yok.kcad"},
     "result": failed("file_not_found", "$any", "path"), "expect": NOTHING},
])
add("çizim olmayan dosya", [
    {"op": "execute", "input": {"path": "$fixture:blocks/v1/attribute-table.json"},
     "result": failed("import_unreadable", "$any", "path"), "expect": NOTHING},
])
add("beklenen sürüm dosyadan sonra denetlenir", [
    {"op": "execute", "input": {"path": FILE, "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                                "path": "expectedRevision", "revision": "$current"}},
     "expect": NOTHING},
])
add("geçersiz beklenen sürüm", [
    {"op": "execute", "input": {"path": FILE, "expectedRevision": "dün"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": NOTHING},
])
add("dosyada olmayan blok adı", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": ["Pencere", "Merdiven"]},
     "result": failed("unknown_block", "“Merdiven” bloğu ofis-bloklari.kcad içinde yok.", "names[1]"), "expect": NOTHING},
])


add("boş ad listesi", [
    {"op": "execute", "input": {"path": FILE, "mode": "blocks", "names": []},
     "result": failed("nothing_to_import", "Alınacak blok adı verilmedi; names en az bir ad ister.", "names"),
     "expect": NOTHING},
])


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "file.import.block",',
           '  "commandVersion": 1,', '  "title": "Dosyadan blok al: çizim tek blok, bloklar adlarıyla, yeniden tanımlama",',
           f'  "note": {compact("Written by scripts/fixtures/block_file_cases.py from docs/adr/0316 §3 and the exchange reference (exchange_cases.py); do not edit by hand.")},',
           f'  "setup": {compact(OURS)},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st_)}" for st_ in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    files = {SOURCE: json.dumps(THEIRS, ensure_ascii=False, indent=1) + "\n", OUT: build()}
    if "--check" in sys.argv[1:]:
        for path, text in files.items():
            if path.read_text(encoding="utf-8") != text:
                sys.exit(f"{path.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    for path, text in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


main()
