#!/usr/bin/env python3
"""The shared cases of `cad.xrefs.edit` and `cad.xrefs.list` (docs/adr/0348; plan/cad.md CAD-REF-05…14).

    python3 scripts/fixtures/xref_command_cases.py           # writes the files
    python3 scripts/fixtures/xref_command_cases.py --check   # writes nothing; compares

Writes the source drawings fixtures/xref/v1/{plan,plan2,bos}.kcad (KCAD v2, through the independent writer
scripts/fixtures/kcad_v2_reference.py) and fixtures/commands/v1/cad.xrefs.{edit,list}.json, with Python's standard
library, no KentOS code. The layer and object rules are the drawing exchange's independent reference
(scripts/fixtures/exchange_cases.py, docs/adr/0193); this script adds the reference's own rules from ADR 0348:

- the source's layers under the group “Dış başvuru: ad”, every layer path taken, a path the drawing has kept as it is;
- its objects but pictures, rasters and tables (said), and but the inserts of its overlaid references, the
  definition's in the source's order, numbered from 1, their persistent ids beside them (`objects`); on a reload an
  object the definition had keeps its local id, a new one takes the next after the highest;
- the blocks they place renamed `ad|blok` (attached references among them keep their source, `parent` this one);
- reload and repath: the definition and what it brought replaced in place, missing layers added;
- unload: the content and what it brought gone, the inserts kept; detach: all of it and the inserts gone, the layer
  group too when nothing is drawn on it; bind: the definitions ordinary blocks, `insert` their layers joined to the
  drawing's of the same path;
- the list: each reference's state from its file (its path, else its relative path from the drawing's folder).

A definition's id the case cannot know is `$blockOf:ad`; a source file's path is `$fixture:xref/v1/…`.
"""
import copy
import hashlib
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import exchange_cases as x  # noqa: E402  (the exchange's own reference, no KentOS code)
import kcad_v2_reference as k  # noqa: E402  (the independent KCAD v2 writer)

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "fixtures" / "xref" / "v1"
OUT = ROOT / "fixtures" / "commands" / "v1"


def P(x_, y):
    return {"x": float(x_), "y": float(y)}


def st(color, weight=0.25):
    return {"color": color, "lineType": "continuous", "lineWeight": weight}


def layer(i, name, style, children=None, kind="layer", visible=True, locked=False, expanded=True):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": expanded,
            "style": style, "children": children or []}


def obj(kind, i, layer_id, **f):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **f}


def line(i, layer_id, a, b):
    return obj("line", i, layer_id, a=P(*a), b=P(*b))


def insert(i, layer_id, block, at, scale=1.0, rotation=0.0):
    return obj("insert", i, layer_id, block=block, p=P(*at), scale=scale, rotation=rotation)


def uid(n):
    return f"0192f5a1-5555-7000-8000-{n:012d}"


SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50.0,
            "workspace": "cad"}

KOLON = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7101"
DETAY = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7102"
CEVRE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7103"
D_DETAY = "1d" * 32
D_CEVRE = "ce" * 32


def source(name, *, moved=False):
    """The architect's plan (and, `moved`, its next version: a wall gone, the door's swing grown, a stair added, the
    column bigger)."""
    layers = [
        layer("0", "0", st("fg")),
        layer("mimari", "Mimari", st("fg"), [
            layer("duvar", "Duvar", st("#C0C0C0", 0.5)),
            layer("kapi", "Kapı", st("#C8A165")),
        ] + ([layer("merdiven", "Merdiven", st("#4A90D9"))] if moved else []), kind="group"),
        layer("yazi", "Yazı", st("fg", 0.18)),
        layer("xr-detay", "Dış başvuru: Detay", st("fg"), [layer("xr-detay-d", "Detay", st("#7A7A7A"))], kind="group"),
        layer("xr-cevre", "Dış başvuru: Çevre", st("fg"), [layer("xr-cevre-c", "Çevre", st("#5BA05B"))], kind="group"),
    ]
    rows = [
        (1, line(0, "duvar", (0, 0), (10, 0))),
        (2, line(0, "duvar", (10, 0), (10, 8))),
        (3, obj("circle", 0, "kapi", c=P(2, 1), r=1.0 if moved else 0.9)),
        (4, obj("text", 0, "yazi", p=P(2, 6), text="Salon", height=0.5, rotation=0.0)),
        (5, insert(0, "duvar", KOLON, (5, 4))),
        (6, insert(0, "0", DETAY, (12, 0))),
        (7, insert(0, "0", CEVRE, (0, -20))),
    ]
    if moved:
        rows = [r for r in rows if r[0] != 2] + [(9, line(0, "merdiven", (6, 2), (9, 2)))]
    else:
        rows.append((8, {**obj("table", 0, "yazi"), "p": P(0, 10), "rotation": 0.0, "height": 0.5, "rows": [1.0],
                         "columns": [4.0], "cells": [["Oda"]]}))
    side = 0.4 if moved else 0.3
    blocks = [
        {"id": KOLON, "name": "Kolon", "base": P(0, 0), "entities": [
            obj("polygon", 1, "", pts=[P(0, 0), P(side, 0), P(side, side), P(0, side)])]},
        {"id": DETAY, "name": "Detay", "base": P(0, 0),
         "xref": {"path": "/proje/detay.kcad", "relative": "detay.kcad", "digest": D_DETAY, "group": "xr-detay",
                  "objects": [uid(101)]},
         "units": "m", "entities": [line(1, "xr-detay-d", (0, 0), (1, 1))]},
        {"id": CEVRE, "name": "Çevre", "base": P(0, 0),
         "xref": {"path": "/proje/cevre.kcad", "kind": "overlay", "digest": D_CEVRE, "group": "xr-cevre",
                  "objects": [uid(201)]},
         "units": "m", "entities": [line(1, "xr-cevre-c", (0, 0), (30, 0))]},
    ]
    entities = []
    for k_, (n, e) in enumerate(rows):
        e = copy.deepcopy(e)
        e["id"] = k_ + 1
        entities.append(e)
    return {"format": "kentos.document", "version": 2, "name": name, "settings": SETTINGS, "origin": P(0, 0),
            "layers": layers, "activeLayer": "duvar", "entities": entities,
            "styles": {"items": [], "categories": []}, "blocks": blocks, "uids": [uid(n) for n, _ in rows]}


def empty_source():
    """A drawing with nothing a block may hold: one table."""
    return {"format": "kentos.document", "version": 2, "name": "Boş", "settings": SETTINGS, "origin": P(0, 0),
            "layers": [layer("0", "0", st("fg"))], "activeLayer": "0",
            "entities": [{**obj("table", 1, "0"), "p": P(0, 0), "rotation": 0.0, "height": 0.5, "rows": [1.0],
                          "columns": [4.0], "cells": [["Oda"]]}],
            "styles": {"items": [], "categories": []}, "uids": [uid(301)]}


PLAN, PLAN2, EMPTY = source("Plan"), source("Plan", moved=True), empty_source()
FILES = {name: k.container(k.document(d)) for name, d in (("plan.kcad", PLAN), ("plan2.kcad", PLAN2), ("bos.kcad", EMPTY))}
DIGEST = {name: hashlib.sha256(b).hexdigest() for name, b in FILES.items()}
F = {name: f"$fixture:xref/v1/{name}" for name in FILES}
DRAWING = "$fixture:xref/v1/ana.kcad"  # the open drawing's file (it need not be there)
THEIRS = {"plan.kcad": PLAN, "plan2.kcad": PLAN2}

# ── The drawing they go into ─────────────────────────────────────────────

OURS = {
    "format": "kentos.document", "version": 1, "name": "Pafta", "settings": SETTINGS, "origin": P(0, 0),
    "layers": [
        layer("0", "0", st("fg")),
        layer("cizim", "Çizim", st("fg")),
        layer("mimari", "Mimari", st("fg"), [layer("duvar", "Duvar", st("#E5484D", 0.35))], kind="group"),
        layer("gizli", "Gizli", st("fg"), visible=False),
        layer("kilitli", "Kilitli", st("fg"), locked=True),
    ],
    "activeLayer": "cizim",
    "entities": [line(1, "cizim", (0, 0), (40, 0))],
    "styles": {"items": [], "categories": []},
}


def group_name(name):
    return f"Dış başvuru: {name}"


def blocks_of(doc):
    return doc.setdefault("blocks", [])


def by_id(doc):
    return {b["id"]: b for b in doc.get("blocks", [])}


def load(ours, theirs, name, ident, xref):
    """ADR 0348 §1–§2: `theirs` as the reference `name` of `ours` (its id `ident`); what the answer says beside it.
    None when nothing may go in the block."""
    ours = copy.deepcopy(ours)
    mine = by_id(ours)
    # A reload keeps each source object's local id; a new one takes the next after the highest (ADR 0348 §1).
    old_ids = {}
    if ident in mine:
        old_ids = dict(zip(mine[ident].get("xref", {}).get("objects", []), (e["id"] for e in mine[ident]["entities"])))
    # A reload: the definition and what it brought go, but a block another object of ours places.
    old = set()
    if ident in mine:
        old = x.placed_blocks(mine[ident]["entities"], mine)
    old |= {b["id"] for b in ours.get("blocks", []) if b.get("xref", {}).get("parent") == ident}
    outside = [e for e in ours["entities"] if not (e["kind"] == "insert" and e["block"] == ident)]
    old -= x.placed_blocks(outside, mine)
    if "blocks" in ours:
        ours["blocks"] = [b for b in ours["blocks"] if b["id"] != ident and b["id"] not in old]
    skip = {b["id"] for b in theirs.get("blocks", []) if b.get("xref", {}).get("kind") == "overlay"}
    kept, kept_uids, left = [], [], 0
    for e, u in zip(theirs["entities"], theirs["uids"]):
        if e["kind"] in ("image", "raster", "table"):
            left += 1
        elif e["kind"] == "insert" and e["block"] in skip:
            pass
        else:
            kept.append(e)
            kept_uids.append(u)
    if not kept:
        return None
    gname = group_name(name)
    wrapped = {"layers": [layer("xref-group", gname, st("fg"), copy.deepcopy(theirs["layers"]), kind="group")]}
    their_paths = x.path_of(wrapped["layers"])
    wanted = {" / ".join(here) for n, here in x.walk(wrapped["layers"]) if n["type"] == "layer"}
    before = x.all_ids(ours["layers"])
    x.take_layers(ours, wrapped, wanted, "skip", set(before))
    group_id = next(n["id"] for n in ours["layers"] if x.fold(n["name"]) == x.fold(gname))
    theirs_by_id = by_id(theirs)
    placed = x.placed_blocks(kept, theirs_by_id) - skip
    brought = [b for b in theirs.get("blocks", []) if b["id"] in placed]
    names = x.unique_names([b["name"] for b in ours.get("blocks", [])], [f"{name}|{b['name']}" for b in brought])
    block_of = {b["id"]: "$blockOf:" + n for b, n in zip(brought, names)}

    def layer_of(i):
        return x.our_node_for(ours, their_paths, i) or ""

    def group_of(i):
        return x.our_node_for(ours, their_paths, i, layers_only=False)

    nested = []
    for b, bname in zip(brought, names):
        b = copy.deepcopy(b)
        b["id"] = block_of[b["id"]]
        b["name"] = bname
        b["entities"] = [x.map_object(e, layer_of, block_of, {}) for e in b["entities"]
                         if not (e["kind"] == "insert" and e["block"] in skip)]
        if "xref" in b:
            b["xref"]["parent"] = ident
            g = group_of(b["xref"].get("group"))
            if g is None:
                b["xref"].pop("group", None)
            else:
                b["xref"]["group"] = g
        nested.append(bname)
        blocks_of(ours).append(b)
    entities, top = [], max(old_ids.values(), default=0)
    for e, u in zip(kept, kept_uids):
        m = x.map_object(e, layer_of, block_of, {})
        if u in old_ids:
            m["id"] = old_ids[u]
        else:
            top += 1
            m["id"] = top
        entities.append(m)
    source = {k_: v for k_, v in xref.items() if v is not None}
    source["group"] = group_id
    source["objects"] = kept_uids
    blocks_of(ours).append({"name": name, "id": ident, "base": P(0, 0), "entities": entities,
                            "units": theirs["settings"].get("drawingUnit", "m"), "xref": source})
    # A definition the drawing had keeps its place; the new ones follow in their order.
    had = [b["id"] for b in mine.values()]
    blocks = ours["blocks"]
    ours["blocks"] = sorted(blocks, key=lambda b: (0, had.index(b["id"])) if b["id"] in had else (1, blocks.index(b)))
    made = [n["name"] for n, _ in x.walk(ours["layers"]) if n["type"] == "layer" and n["id"] not in before]
    return ours, {"objects": len(entities), "left": left, "layers": made, "nested": nested}


def xref_of(path_key, kind="attach", relative=None):
    out = {"path": F[path_key], "digest": DIGEST[path_key]}
    if relative is not None:
        out["relative"] = relative
    if kind == "overlay":
        out["kind"] = "overlay"
    return out


def without_ids(doc):
    out = []
    for b in doc.get("blocks", []):
        b = copy.deepcopy(b)
        b.pop("id")
        out.append(b)
    return out


def brought_by(doc, ident):
    """The definitions `ident` brought: those its content places and the references whose `parent` it is."""
    mine = by_id(doc)
    out = x.placed_blocks(mine[ident]["entities"], mine)
    out |= {b["id"] for b in doc.get("blocks", []) if b.get("xref", {}).get("parent") == ident}
    return out


def group_unused(doc, g):
    leaves = {n["id"] for n, _ in x.walk([find_node(doc["layers"], g)]) if n["type"] == "layer"}
    drawn = any(e["layerId"] in leaves for e in doc["entities"])
    pieces = any(e["layerId"] in leaves for b in doc.get("blocks", []) for e in b["entities"])
    return not (drawn or pieces)


def find_node(tree, i):
    return next(n for n, _ in x.walk(tree) if n["id"] == i)


def remove_node(tree, i):
    for k_, n in enumerate(tree):
        if n["id"] == i:
            del tree[k_]
            return True
        if remove_node(n["children"], i):
            return True
    return False


def unload(doc, ident):
    doc = copy.deepcopy(doc)
    gone = brought_by(doc, ident)
    for b in doc["blocks"]:
        if b["id"] == ident:
            b["entities"] = []
            b["xref"]["unloaded"] = True
            b["xref"].pop("objects", None)
    doc["blocks"] = [b for b in doc["blocks"] if b["id"] not in gone]
    return doc


def detach(doc, ident):
    doc = copy.deepcopy(doc)
    gone = brought_by(doc, ident) | {ident}
    g = by_id(doc)[ident]["xref"].get("group")
    removed = sum(1 for e in doc["entities"] if e["kind"] == "insert" and e["block"] == ident)
    doc["entities"] = [e for e in doc["entities"] if not (e["kind"] == "insert" and e["block"] == ident)]
    doc["blocks"] = [b for b in doc["blocks"] if b["id"] not in gone]
    if not doc["blocks"]:
        del doc["blocks"]
    if g is not None and group_unused(doc, g):
        remove_node(doc["layers"], g)
    return doc, removed


def bind(doc, ident, mode):
    """ADR 0348 §3: the definitions ordinary blocks; `insert` their layers joined to the drawing's of the same path."""
    doc = copy.deepcopy(doc)
    defs = [ident] + sorted(brought_by(doc, ident), key=[b["id"] for b in doc["blocks"]].index)
    g = by_id(doc)[ident]["xref"].get("group")
    merged = []
    if mode == "insert" and g is not None:
        theirs = {"layers": copy.deepcopy(find_node(doc["layers"], g)["children"])}
        their_paths = x.path_of(theirs["layers"])
        wanted = {" / ".join(here) for n, here in x.walk(theirs["layers"]) if n["type"] == "layer"}
        had = {i for i in their_paths if x.our_node_for(doc, their_paths, i) is not None}
        x.take_layers(doc, theirs, wanted, "skip", x.all_ids(doc["layers"]))
        to = {}
        for n, _ in x.walk(theirs["layers"]):
            if n["type"] != "layer":
                continue
            ours = x.our_node_for(doc, their_paths, n["id"])
            if ours is not None:
                if n["id"] in had:
                    merged.append(n["name"])
                to[n["id"]] = ours
        for b in doc["blocks"]:
            if b["id"] in defs:
                for e in b["entities"]:
                    e["layerId"] = to.get(e["layerId"], e["layerId"])
    for b in doc["blocks"]:
        if b["id"] in defs:
            b.pop("xref", None)
    if mode == "insert" and g is not None and group_unused(doc, g):
        remove_node(doc["layers"], g)
    return doc, merged


def attach_insert(slot, ident, layer_id, at=(0, 0), scale=1.0, rotation=0.0):
    return {"kind": "insert", "id": slot, "layerId": layer_id, "attrs": {}, "block": ident, "p": P(*at),
            "scale": scale, "rotation": rotation}


# ── Steps ────────────────────────────────────────────────────────────────

def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


NOTHING = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
LEFT = {"code": "objects_left_out", "message": "1 resim, raster ya da tablo bloğa konamadığı için alınmadı.",
        "path": "operation"}
PLAN_ID = "$blockOf:Plan"


def edited(name, *, block=None, insert=False, objects=0, layers=(), nested=(), removed=0, merged=()):
    out = {"name": name, "objects": objects, "removed": removed, "revision": "$current"}
    if block is not None:
        out["block"] = block
    if insert:
        out["insert"] = "$any"
    if layers:
        out["layers"] = list(layers)
    if nested:
        out["nested"] = list(nested)
    if merged:
        out["merged"] = list(merged)
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def attached(ours, key="plan.kcad", name="Plan", ident=PLAN_ID, kind="attach", relative=None, layer_id="cizim",
             at=(0, 0), scale=1.0, rotation=0.0):
    """The drawing after attaching `key` as `name`, the answer, and the insert."""
    after, info = load(ours, THEIRS[key], name, ident, xref_of(key, kind, relative))
    slot = max([e["id"] for e in ours["entities"]] + [0]) + 1
    after["entities"].append(attach_insert(slot, ident, layer_id, at, scale, math.radians(rotation)))
    answer = edited(name, block=ident, insert=True, objects=info["objects"], layers=info["layers"],
                    nested=info["nested"])
    return after, answer, info


def expect(doc, **more):
    out = {"blocks": without_ids(doc), "layers": doc["layers"], "ids": [e["id"] for e in doc["entities"]]}
    out.update(more)
    return out


edit_cases, list_cases = [], []


def add(cases, name, steps, setup=None, note=None):
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    if note:
        c["note"] = note
    cases.append(c)


def attach_op(path_key="plan.kcad", **f):
    return {"kind": "attach", "path": F[path_key], "layerId": f.pop("layerId", "cizim"), **f}


ATTACHED, ANSWER, INFO = attached(OURS)
assert INFO["left"] == 1 and INFO["nested"] == ["Plan|Kolon", "Plan|Detay"], INFO

# ── attach ───────────────────────────────────────────────────────────────

add(edit_cases, "ekle: kaynağın katmanları “Dış başvuru: Plan” grubunda, nesneleri tanımda, iç blok ve iç içe dış başvuru adıyla; tek adımda geri alınır", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan")}, "result": done(ANSWER, [LEFT]),
     "expect": expect(ATTACHED, canUndo=True, dirty=True, revision="changed")},
    {"op": "undo", "returns": "Dış başvuru ekle: Plan", "expect": expect(OURS)},
], note="Tablo bloğa konamaz, söylenir; Bindir olan Çevre ve yerleştirmesi gelmez; Mimari/Duvar zaten bu çizimdekiyle aynı yolda değil (grubun altında), yeniden açılır.")

after_default, answer_default, _ = attached(OURS, name="plan")
add(edit_cases, "ekle: ad verilmezse dosyanın adı", [
    {"op": "execute", "input": {"operation": attach_op()}, "result": done(answer_default, [LEFT]),
     "expect": expect(after_default)},
])
after_blank, answer_blank, _ = attached(OURS, name="plan")
add(edit_cases, "ekle: boş ad dosyanın adına döner", [
    {"op": "execute", "input": {"operation": attach_op(name="  ")}, "result": done(answer_blank, [LEFT]),
     "expect": expect(after_blank)},
])
after_at, answer_at, _ = attached(OURS, at=(100, 50), scale=2.0, rotation=90.0)
add(edit_cases, "ekle: nokta, ölçek ve derece cinsinden dönüş yerleştirmeye", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", at=P(100, 50), scale=2.0, rotation=90.0)},
     "result": done(answer_at, [LEFT]), "expect": expect(after_at, entities={"2": after_at["entities"][-1]})},
])
after_overlay, answer_overlay, _ = attached(OURS, kind="overlay")
add(edit_cases, "ekle: Bindir türü kaynağında yazılır", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", mode="overlay")},
     "result": done(answer_overlay, [LEFT]), "expect": expect(after_overlay)},
])
after_rel, answer_rel, _ = attached(OURS, relative="plan.kcad")
add(edit_cases, "ekle: açık çizimin dosyası verilince göreli yol yazılır", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "drawing": DRAWING},
     "result": done(answer_rel, [LEFT]), "expect": expect(after_rel)},
])
after_up, answer_up, _ = attached(OURS, relative="../plan.kcad")
add(edit_cases, "ekle: alt klasördeki çizimden göreli yol bir üst klasöre", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "drawing": "$fixture:xref/v1/proje/ana.kcad"},
     "result": done(answer_up, [LEFT]), "expect": expect(after_up)},
])
after_hidden, answer_hidden, _ = attached(OURS, layer_id="gizli")
add(edit_cases, "ekle: gizli katmana yerleştirme uyarıyla", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", layerId="gizli")},
     "result": done(answer_hidden, [{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                                     "path": "operation.layerId"}, LEFT]),
     "expect": expect(after_hidden)},
])
TAKEN = {**copy.deepcopy(OURS), "blocks": [{"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7201", "name": "Plan", "base": P(0, 0),
                                             "entities": [line(1, "", (0, 0), (1, 0))]}]}
after_taken, answer_taken, _ = attached(TAKEN, name="plan (2)", ident="$blockOf:plan (2)")
add(edit_cases, "ekle: çizimde olan ad “(2)” alır", [
    {"op": "execute", "input": {"operation": attach_op(name="plan")}, "result": done(answer_taken, [LEFT]),
     "expect": expect(after_taken)},
], setup=TAKEN, note="Ad büyük küçük harfe bakmadan karşılaştırılır; verilen yazılışla “plan (2)”, iç bloklar da: “plan (2)|Kolon”.")
twice, twice_answer, _ = attached(ATTACHED, name="Plan (2)", ident="$blockOf:Plan (2)")
add(edit_cases, "ekle: aynı dosya iki kez, ikincisi kendi grubu ve katmanlarıyla", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan")}, "result": done(ANSWER, [LEFT])},
    {"op": "execute", "input": {"operation": attach_op(name="Plan")}, "result": done(twice_answer, [LEFT]),
     "expect": expect(twice)},
])
add(edit_cases, "ekle: plan yazmaz, yerleştirmeyi adlandırmaz", [
    {"op": "plan", "input": {"operation": attach_op(name="Plan")},
     "result": done({**{k_: v for k_, v in ANSWER.items() if k_ not in ("insert", "block")}, "block": "$any"}, [LEFT]),
     "expect": NOTHING},
])
add(edit_cases, "ekle: doğrulama dosyayı okur, yazmaz", [
    {"op": "validate", "input": {"operation": attach_op(name="Plan")}, "result": done(None), "expect": NOTHING},
])
add(edit_cases, "ekle: doğru sürümle yazılır, geri alınan yinelenir", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "expectedRevision": "$current"},
     "result": done(ANSWER, [LEFT])},
    {"op": "undo", "returns": "Dış başvuru ekle: Plan", "expect": expect(OURS)},
    {"op": "redo", "returns": "Dış başvuru ekle: Plan", "expect": expect(ATTACHED, canRedo=False)},
])

# ── reload, repath ───────────────────────────────────────────────────────

ATTACH = {"op": "execute", "input": {"operation": attach_op(name="Plan")}, "result": done(ANSWER, [LEFT])}
reloaded, info_r = load(ATTACHED, PLAN, "Plan", PLAN_ID, xref_of("plan.kcad"))
assert without_ids(reloaded) == without_ids(ATTACHED) and reloaded["layers"] == ATTACHED["layers"]
add(edit_cases, "yeniden yükle: dosya değişmediyse hiçbir şey yazılmaz, geri alma adımı olmaz", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "reload", "name": "plan"}},
     "result": done(edited("Plan", block=PLAN_ID, objects=info_r["objects"], nested=info_r["nested"]), [LEFT]),
     "expect": expect(reloaded, revision="same")},
    {"op": "undo", "returns": "Dış başvuru ekle: Plan", "expect": expect(OURS)},
], note="Ad büyük küçük harfe bakmadan bulunur.")
repathed, info_p = load(ATTACHED, PLAN2, "Plan", PLAN_ID, xref_of("plan2.kcad"))
add(edit_cases, "yolu değiştir: yeni dosyanın içeriği, eksik katmanı eklenir, değişen iç blok yerinde", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "repath", "name": "Plan", "path": F["plan2.kcad"]}},
     "result": done(edited("Plan", block=PLAN_ID, objects=info_p["objects"], layers=info_p["layers"],
                           nested=info_p["nested"])),
     "expect": expect(repathed)},
    {"op": "undo", "returns": "Dış başvurunun yolunu değiştir: Plan", "expect": expect(ATTACHED)},
], note="Duvarın biri gitti, kapının yayı büyüdü, merdiven eklendi; kolon büyüdü; tablo yok. Kalan nesneler yerel kimliklerini korur (1, 3, 4, 5, 6), merdiven 7 olur.")
assert info_p["layers"] == ["Merdiven"], info_p
coloured = copy.deepcopy(ATTACHED)
add(edit_cases, "yolu değiştir: göreli yol açık çizimin klasöründen yeniden yazılır", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "repath", "name": "Plan", "path": F["plan2.kcad"]},
                                "drawing": DRAWING},
     "result": done(edited("Plan", block=PLAN_ID, objects=info_p["objects"], layers=info_p["layers"],
                           nested=info_p["nested"])),
     "expect": expect(load(ATTACHED, PLAN2, "Plan", PLAN_ID, xref_of("plan2.kcad", relative="plan2.kcad"))[0])},
])

# ── unload, detach ───────────────────────────────────────────────────────

unloaded = unload(ATTACHED, PLAN_ID)
add(edit_cases, "boşalt: içerik ve getirdikleri gider, yerleştirme ve katmanlar kalır", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "unload", "name": "Plan"}},
     "result": done(edited("Plan", block=PLAN_ID)), "expect": expect(unloaded)},
    {"op": "undo", "returns": "Dış başvuruyu boşalt: Plan", "expect": expect(ATTACHED)},
])
back, info_b = load(unloaded, PLAN, "Plan", PLAN_ID, xref_of("plan.kcad"))
add(edit_cases, "boşaltılan yeniden yüklenince içeriği döner", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "unload", "name": "Plan"}}, "result": done(edited("Plan", block=PLAN_ID))},
    {"op": "execute", "input": {"operation": {"kind": "reload", "name": "Plan"}},
     "result": done(edited("Plan", block=PLAN_ID, objects=info_b["objects"], nested=info_b["nested"]), [LEFT]),
     "expect": expect(back)},
])
detached, removed = detach(ATTACHED, PLAN_ID)
assert detached["layers"] == OURS["layers"] and removed == 1
add(edit_cases, "kaldır: tanım, getirdikleri, yerleştirme ve boş kalan katman grubu gider", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "detach", "name": "Plan"}},
     "result": done(edited("Plan", removed=removed)), "expect": expect(detached)},
    {"op": "undo", "returns": "Dış başvuruyu kaldır: Plan", "expect": expect(ATTACHED)},
])

# A drawing with the reference already in it (its ids fixed), and an object of ours on one of its layers.
find_layer_id = next(n["id"] for n, h in x.walk(ATTACHED["layers"]) if h == ("Dış başvuru: Plan", "Yazı"))
SETUP_IDS = {"$blockOf:Plan": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7301", "$blockOf:Plan|Kolon": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7302",
             "$blockOf:Plan|Detay": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7303"}


def with_ids(v):
    if isinstance(v, str):
        return SETUP_IDS.get(v, v)
    if isinstance(v, list):
        return [with_ids(i) for i in v]
    if isinstance(v, dict):
        return {k_: with_ids(i) for k_, i in v.items() if i is not None}
    return v


DRAWN = with_ids(copy.deepcopy(ATTACHED))
DRAWN["entities"].append(line(3, find_layer_id, (0, 5), (4, 5)))
kept_group, removed_k = detach(DRAWN, SETUP_IDS[PLAN_ID])
assert find_layer_id in x.all_ids(kept_group["layers"])
add(edit_cases, "kaldır: grubun katmanında bu çizimin nesnesi varsa grup kalır", [
    {"op": "execute", "input": {"operation": {"kind": "detach", "name": "Plan"}},
     "result": done(edited("Plan", removed=removed_k)), "expect": expect(kept_group)},
], setup=DRAWN, note="Kaldırmak bu çizimin nesnelerini silmez.")

# ── bind ─────────────────────────────────────────────────────────────────

bound, _ = bind(ATTACHED, PLAN_ID, "bind")
add(edit_cases, "bağla: dış başvuru ve getirdiği sıradan blok olur, katmanlar grubunda kalır", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "bind", "name": "Plan"}},
     "result": done(edited("Plan")), "expect": expect(bound)},
    {"op": "undo", "returns": "Dış başvuruyu bağla: Plan", "expect": expect(ATTACHED)},
])
inserted, merged = bind(ATTACHED, PLAN_ID, "insert")
add(edit_cases, "bağla, yerleştir: katmanlar çizimin aynı yoldaki katmanlarıyla birleşir, grup gider", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "bind", "name": "Plan", "mode": "insert"}},
     "result": done(edited("Plan", merged=merged)), "expect": expect(inserted)},
], note="0 ve Mimari/Duvar bu çizimde var: birleşir, renkleri bu çizimin; diğerleri açılır.")
assert merged == ["0", "Duvar"], merged
inserted_k, merged_k = bind(DRAWN, SETUP_IDS[PLAN_ID], "insert")
add(edit_cases, "bağla, yerleştir: grubun katmanında bu çizimin nesnesi varsa grup kalır", [
    {"op": "execute", "input": {"operation": {"kind": "bind", "name": "Plan", "mode": "insert"}},
     "result": done(edited("Plan", merged=merged_k)), "expect": expect(inserted_k)},
], setup=DRAWN)

# ── Refusals ─────────────────────────────────────────────────────────────

add(edit_cases, "göreli yol", [
    {"op": "execute", "input": {"operation": {"kind": "attach", "path": "plan.kcad", "layerId": "cizim"}},
     "result": failed("invalid_file_path", "“plan.kcad” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).",
                      "operation.path"), "expect": NOTHING},
])
add(edit_cases, "olmayan dosya", [
    {"op": "execute", "input": {"operation": {"kind": "attach", "path": "$fixture:xref/v1/yok.kcad", "layerId": "cizim"}},
     "result": failed("file_not_found", "$any", "operation.path"), "expect": NOTHING},
])
add(edit_cases, "çizim olmayan dosya", [
    {"op": "execute", "input": {"operation": {"kind": "attach", "path": "$fixture:blocks/v1/attribute-table.json",
                                              "layerId": "cizim"}},
     "result": failed("import_unreadable", "$any", "operation.path"), "expect": NOTHING},
])
add(edit_cases, "bloğa konacak nesnesi olmayan çizim", [
    {"op": "execute", "input": {"operation": attach_op("bos.kcad", name="Boş")},
     "result": failed("xref_empty", "“Boş” içinde dış başvuruya konacak nesne yok; resimler, rasterler ve tablolar bloğa konamaz.",
                      "operation.path"), "expect": NOTHING},
])
add(edit_cases, "açık çizimin kendisi: döngü", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "drawing": F["plan.kcad"]},
     "result": failed("xref_cycle", "plan.kcad açık çizimin kendisi; çizim kendine dış başvuru olamaz.", "operation.path"),
     "expect": NOTHING},
])
add(edit_cases, "olmayan katman", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", layerId="yok")},
     "result": failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "operation.layerId"), "expect": NOTHING},
])
add(edit_cases, "katman grubu", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", layerId="mimari")},
     "result": failed("not_a_layer", "“Mimari” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
                      "operation.layerId"), "expect": NOTHING},
])
add(edit_cases, "kilitli katman", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", layerId="kilitli")},
     "result": failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                      "operation.layerId"), "expect": NOTHING},
])
add(edit_cases, "sonsuz nokta", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", at=P(0, 0))}, "nonFinite": {"operation.at.x": "Infinity"},
     "result": failed("not_finite", "Dış başvurunun noktası sayı değil ya da sonsuz.", "operation.at"), "expect": NOTHING},
])
add(edit_cases, "sıfır ölçek", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan", scale=0.0)},
     "result": failed("invalid_scale", "Dış başvurunun ölçeği sıfırdan büyük ve en çok 1 000 000 olmalı.", "operation.scale"),
     "expect": NOTHING},
])
add(edit_cases, "adı olmayan dış başvuru", [
    {"op": "execute", "input": {"operation": {"kind": "reload", "name": "Yok"}},
     "result": failed("xref_not_found", "“Yok” adlı dış başvuru yok. Dış başvurular penceresindeki adı verin.", "operation.name"),
     "expect": NOTHING},
])
add(edit_cases, "sıradan blok dış başvuru değildir", [
    {"op": "execute", "input": {"operation": {"kind": "unload", "name": "Plan"}},
     "result": failed("xref_not_found", "“Plan” adlı dış başvuru yok. Dış başvurular penceresindeki adı verin.", "operation.name"),
     "expect": {"ids": [1], "canUndo": False, "revision": "same"}},
], setup=TAKEN)
for kind in ("reload", "unload", "detach"):
    add(edit_cases, f"iç içe gelen dış başvuru kendi başına: {kind}", [
        ATTACH,
        {"op": "execute", "input": {"operation": {"kind": kind, "name": "Plan|Detay"}},
         "result": failed("nested_xref", "“Plan|Detay” dış başvurusunu “Plan” getirdi; kendi başına değiştirilemez. Onu getireni yeniden yükleyin, boşaltın ya da kaldırın.",
                          "operation.name"),
         "expect": expect(ATTACHED, revision="same")},
    ])
add(edit_cases, "yolu değiştir: olmayan dosya", [
    ATTACH,
    {"op": "execute", "input": {"operation": {"kind": "repath", "name": "Plan", "path": "$fixture:xref/v1/yok.kcad"}},
     "result": failed("file_not_found", "$any", "operation.path"), "expect": expect(ATTACHED, revision="same")},
])
add(edit_cases, "beklenen sürüm dosyadan sonra denetlenir", [
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "expectedRevision": "999"},
     "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                                "path": "expectedRevision", "revision": "$current"}},
     "expect": NOTHING},
])
add(edit_cases, "geçersiz beklenen sürüm", [
    {"op": "execute", "input": {"operation": {"kind": "unload", "name": "Yok"}, "expectedRevision": "dün"},
     "result": failed("xref_not_found", "$any", "operation.name"), "expect": NOTHING},
    {"op": "execute", "input": {"operation": attach_op(name="Plan"), "expectedRevision": "dün"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": NOTHING},
], note="Ad sürümden önce denetlenir.")

# ── search paths (ADR 0349 §1) ──────────────────────────────────────────


def paths_done(paths):
    out = {"name": "", "objects": 0, "removed": 0, "revision": "$current"}
    if paths:
        out["paths"] = paths
    return done(out)


add(edit_cases, "arama yolları: kırpılır, boş ve yinelenen düşer, ayardır", [
    {"op": "execute", "input": {"operation": {"kind": "paths", "paths": [" /ortak/mimari ", "", "../referanslar", "/ortak/mimari"]}},
     "result": paths_done(["/ortak/mimari", "../referanslar"]),
     "expect": {"settings": {"xrefPaths": ["/ortak/mimari", "../referanslar"]}, "canUndo": False}},
], note="Ayar geri alma adımı değildir (cad.settings.set gibi).")
add(edit_cases, "arama yolları boş listeyle kalkar", [
    {"op": "execute", "input": {"operation": {"kind": "paths", "paths": ["/ortak"]}}, "result": paths_done(["/ortak"])},
    {"op": "execute", "input": {"operation": {"kind": "paths", "paths": []}}, "result": paths_done([]),
     "expect": {"settings": {"xrefPaths": None}}},
])
add(edit_cases, "arama yolları: plan yazmaz", [
    {"op": "plan", "input": {"operation": {"kind": "paths", "paths": ["/ortak"]}}, "result": paths_done(["/ortak"]),
     "expect": {"settings": {"xrefPaths": None}}},
])
add(edit_cases, "arama yolları: geçersiz beklenen sürüm", [
    {"op": "execute", "input": {"operation": {"kind": "paths", "paths": ["/ortak"]}, "expectedRevision": "dün"},
     "result": failed("invalid_revision", "$any", "expectedRevision"), "expect": {"settings": {"xrefPaths": None}}},
])
SEARCHED = with_ids(copy.deepcopy(ATTACHED))
for b in SEARCHED["blocks"]:
    if b["name"] == "Plan":
        b["xref"]["path"] = "/yok/plan.kcad"
SEARCHED["settings"]["xrefPaths"] = ["/ortak", F["plan.kcad"].rsplit("/", 1)[0]]
add(edit_cases, "yeniden yükle: kaynak arama yolunda bulunur, kayıtlı yol oraya yazılır", [
    {"op": "execute", "input": {"operation": {"kind": "reload", "name": "Plan"}},
     "result": done(edited("Plan", block=SETUP_IDS[PLAN_ID], objects=INFO["objects"], nested=INFO["nested"]), [LEFT])},
], setup=SEARCHED, note="İçerik aynı; kayıtlı yol bulunan dosyanın yolu olur ve adım yazılır.")
MISSING = copy.deepcopy(SEARCHED)
MISSING["settings"]["xrefPaths"] = ["/ortak"]
add(edit_cases, "yeniden yükle: arama yollarında da yoksa dosya bulunamadı", [
    {"op": "execute", "input": {"operation": {"kind": "reload", "name": "Plan"}},
     "result": failed("file_not_found", "$any", "operation.name"), "expect": {"canUndo": False, "revision": "same"}},
], setup=MISSING)

# ── list ─────────────────────────────────────────────────────────────────


def info(name, block, kind, path, state, *, relative=None, parent=None, inserts=0, objects=0, group=None, via=None,
         tried=(), sheets=(), views=()):
    """A listed reference (ADR 0348 §3, 0349 §1, §3): found at its path unless `via` says otherwise; one not looked at or
    not found says how it was looked for."""
    out = {"name": name, "block": block, "kind": kind, "path": path, "state": state, "inserts": inserts, "objects": objects}
    for k_, v in (("relative", relative), ("parent", parent), ("group", group)):
        if v is not None:
            out[k_] = v
    out["via"] = via or ("none" if state in ("missing", "unloaded") else "path")
    for k_, v in (("tried", tried), ("sheets", sheets), ("views", views)):
        if v:
            out[k_] = list(v)
    return out


def listed(*items):
    return {"status": "completed", "output": {"xrefs": list(items), "revision": "$current"}, "warnings": []}


def setup_with(sources):
    """OURS with references whose sources (path, relative, digest, flags) the case gives, one insert each."""
    d = copy.deepcopy(OURS)
    d["blocks"] = []
    for k_, (name, xs) in enumerate(sources):
        ident = f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d74{k_:02d}"
        unloaded = xs.get("unloaded", False)
        d["blocks"].append({"id": ident, "name": name, "base": P(0, 0),
                            "entities": [] if unloaded else [line(1, "", (0, 0), (1, 0))],
                            "xref": {**xs, **({} if unloaded else {"objects": [uid(900 + k_)]})}})
        if not xs.get("parent"):
            d["entities"].append(insert(len(d["entities"]) + 1, "cizim", ident, (k_ * 20, 0)))
    return d


LOADED = setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"]})])
add(list_cases, "yüklü: dosya yüklenen içerikle aynı", [
    {"op": "execute", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                         F["plan.kcad"], "loaded", inserts=1, objects=1)),
     "expect": {"canUndo": False, "revision": "same"}},
], setup=LOADED)
add(list_cases, "değişti: dosyanın özeti yüklenenden başka", [
    {"op": "execute", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                         F["plan2.kcad"], "changed", inserts=1, objects=1))},
], setup=setup_with([("Plan", {"path": F["plan2.kcad"], "digest": DIGEST["plan.kcad"]})]))
add(list_cases, "bulunamadı: dosya yok, göreli yol da yok", [
    {"op": "execute", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                         "/yok/plan.kcad", "missing", inserts=1, objects=1,
                                                         tried=["/yok/plan.kcad"]))},
], setup=setup_with([("Plan", {"path": "/yok/plan.kcad", "digest": DIGEST["plan.kcad"]})]))
RELATIVE = setup_with([("Plan", {"path": "/yok/plan.kcad", "relative": "plan.kcad", "digest": DIGEST["plan.kcad"]})])
add(list_cases, "göreli yol: dosya yerinde değil, açık çizimin klasöründe bulunur", [
    {"op": "execute", "input": {"drawing": DRAWING},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           relative="plan.kcad", inserts=1, objects=1, via="relative"))},
], setup=RELATIVE)
add(list_cases, "göreli yol: açık çizimin dosyası verilmezse bulunamadı", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", "/yok/plan.kcad", "missing",
                           relative="plan.kcad", inserts=1, objects=1, tried=["/yok/plan.kcad"]))},
], setup=RELATIVE)
add(list_cases, "göreli yol alt klasörden", [
    {"op": "execute", "input": {"drawing": "$fixture:xref/v1/proje/ana.kcad"},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           relative="../plan.kcad", inserts=1, objects=1, via="relative"))},
], setup=setup_with([("Plan", {"path": "/yok/plan.kcad", "relative": "../plan.kcad", "digest": DIGEST["plan.kcad"]})]))
add(list_cases, "boşaltılmış: dosyaya bakılmaz", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", "/yok/plan.kcad", "unloaded",
                           inserts=1, objects=0))},
], setup=setup_with([("Plan", {"path": "/yok/plan.kcad", "digest": DIGEST["plan.kcad"], "unloaded": True})]))
add(list_cases, "Bindir türü", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "overlay", F["plan.kcad"], "loaded",
                           inserts=1, objects=1))},
], setup=setup_with([("Plan", {"path": F["plan.kcad"], "kind": "overlay", "digest": DIGEST["plan.kcad"]})]))
add(list_cases, "iç içe: getireni adıyla, kendi yerleştirmesi yok", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1),
                      info("Plan|Detay", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7401", "attach", "/proje/detay.kcad",
                           "missing", parent="Plan", inserts=0, objects=1, tried=["/proje/detay.kcad"]))},
], setup=setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"]}),
                     ("Plan|Detay", {"path": "/proje/detay.kcad", "digest": D_DETAY,
                                     "parent": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400"})]))
GROUPED = setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"], "group": "xr"})])
GROUPED["layers"].append(layer("xr", "Dış başvuru: Plan", st("fg"), [layer("xr-d", "Duvar", st("fg"))], kind="group"))
add(list_cases, "katman grubu adıyla", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1, group="Dış başvuru: Plan"))},
], setup=GROUPED)
add(list_cases, "silinmiş katman grubu söylenmez", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1))},
], setup=setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"], "group": "yok"})]))
TWO = setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"]}),
                  ("Taşıyıcı", {"path": "/yok/tasiyici.kcad", "digest": DIGEST["plan2.kcad"]})])
TWO["entities"].append(insert(4, "cizim", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", (0, 40)))
add(list_cases, "birkaç dış başvuru çizimin blok sırasıyla, yerleştirmeleri sayılarak", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=2, objects=1),
                      info("Taşıyıcı", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7401", "attach", "/yok/tasiyici.kcad",
                           "missing", inserts=1, objects=1, tried=["/yok/tasiyici.kcad"]))},
], setup=TWO)
add(list_cases, "dış başvurusu olmayan çizim", [
    {"op": "execute", "input": {}, "result": listed()},
])
add(list_cases, "sıradan blok listede yok", [
    {"op": "execute", "input": {}, "result": listed()},
], setup=TAKEN)
add(list_cases, "plan aynı cevabı verir, yazmaz", [
    {"op": "plan", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                      F["plan.kcad"], "loaded", inserts=1, objects=1)),
     "expect": {"canUndo": False, "revision": "same"}},
], setup=LOADED)
add(list_cases, "doğrulama", [
    {"op": "validate", "input": {}, "result": done(None)},
], setup=LOADED)
add(list_cases, "olmayan dosyanın açık çizimi: göreli yol yine bulunamaz", [
    {"op": "execute", "input": {"drawing": "$fixture:xref/yok/ana.kcad"},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", "/yok/plan.kcad", "missing",
                           relative="plan.kcad", inserts=1, objects=1,
                           tried=["/yok/plan.kcad", "$fixture:xref/yok/plan.kcad"]))},
], setup=RELATIVE)
add(list_cases, "yeni eklenen dış başvuru yüklü görünür", [
    {"op": "execute", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                         F["plan.kcad"], "loaded", inserts=1, objects=1))},
], setup=LOADED)
add(list_cases, "değişen yolun göreli yolu varken dosyanın kendisi önce", [
    {"op": "execute", "input": {"drawing": DRAWING},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan2.kcad"], "changed",
                           relative="plan.kcad", inserts=1, objects=1))},
], setup=setup_with([("Plan", {"path": F["plan2.kcad"], "relative": "plan.kcad", "digest": DIGEST["plan.kcad"]})]))

add(list_cases, "değişti: göreli yolda bulunan dosya da özetiyle denetlenir", [
    {"op": "execute", "input": {"drawing": DRAWING},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan2.kcad"], "changed",
                           relative="plan2.kcad", inserts=1, objects=1, via="relative"))},
], setup=setup_with([("Plan", {"path": "/yok/plan2.kcad", "relative": "plan2.kcad", "digest": DIGEST["plan.kcad"]})]))
UNPLACED = setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"]})])
UNPLACED["entities"] = UNPLACED["entities"][:1]
add(list_cases, "yerleştirmesi silinmiş dış başvuru sıfır yerleştirmeyle kalır", [
    {"op": "execute", "input": {}, "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach",
                                                         F["plan.kcad"], "loaded", inserts=0, objects=1))},
], setup=UNPLACED)

add(list_cases, "arama yolu: kayıtlı yolda yok, mutlak arama yolunda adıyla bulunur", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1, via="searchPath"))},
], setup={**setup_with([("Plan", {"path": "/yok/plan.kcad", "digest": DIGEST["plan.kcad"]})]),
          "settings": {**SETTINGS, "xrefPaths": ["/ortak", "$fixture:xref/v1"]}})
add(list_cases, "arama yolu: göreli arama yolunda göreli yoluyla, denenenler sırasıyla değil bulunan", [
    {"op": "execute", "input": {"drawing": "$fixture:xref/ana.kcad"},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           relative="plan.kcad", inserts=1, objects=1, via="searchPath"))},
], setup={**setup_with([("Plan", {"path": "/yok/plan.kcad", "relative": "plan.kcad", "digest": DIGEST["plan.kcad"]})]),
          "settings": {**SETTINGS, "xrefPaths": ["eski", "v1"]}})
add(list_cases, "arama yolu: hiçbir yerde yok, denenen yollar sırasıyla", [
    {"op": "execute", "input": {"drawing": "$fixture:xref/ana.kcad"},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", "/yok/mimari.kcad", "missing",
                           relative="mimari.kcad", inserts=1, objects=1,
                           tried=["/yok/mimari.kcad", "$fixture:xref/mimari.kcad", "/ortak/mimari.kcad",
                                  "$fixture:xref/eski/mimari.kcad"]))},
], setup={**setup_with([("Plan", {"path": "/yok/mimari.kcad", "relative": "mimari.kcad", "digest": DIGEST["plan.kcad"]})]),
          "settings": {**SETTINGS, "xrefPaths": ["/ortak", "eski"]}},
   note="Göreli yol ile dosyanın adı aynı yeri gösterince yer bir kez denenir.")
VIEWS = setup_with([("Plan", {"path": F["plan.kcad"], "digest": DIGEST["plan.kcad"]}),
                    ("Plan|Detay", {"path": "/proje/detay.kcad", "digest": D_DETAY,
                                    "parent": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400"})])
VIEWS["settings"] = {**SETTINGS, "namedViews": [
    {"name": "Giriş", "center": P(0, 0), "width": 10.0, "height": 10.0, "twist": 0.0},
    {"name": "Uzak", "center": P(500, 500), "width": 10.0, "height": 10.0, "twist": 0.0},
    {"name": "Dönük", "center": P(7, 0), "width": 12.0, "height": 2.0, "twist": 1.5707963267948966}]}
add(list_cases, "etkilenenler: yerleştirmenin kutusuyla kesişen adlı görünümler, iç içe olan getireninkiyle", [
    {"op": "execute", "input": {"affected": True},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1, views=["Giriş"]),
                      info("Plan|Detay", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7401", "attach", "/proje/detay.kcad",
                           "missing", parent="Plan", inserts=0, objects=1, tried=["/proje/detay.kcad"], views=["Giriş"]))},
], setup=VIEWS, note="Dönük görünümün penceresi 2 × 12 dikey şerit, x 6–8: yerleştirmenin 0–1'lik çizgisine değmez. Paftaların etkilenmesi cad.sheets.set'in shows kuralıdır (kendi durumları).")
add(list_cases, "etkilenenler istenmezse söylenmez", [
    {"op": "execute", "input": {},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=1, objects=1),
                      info("Plan|Detay", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7401", "attach", "/proje/detay.kcad",
                           "missing", parent="Plan", inserts=0, objects=1, tried=["/proje/detay.kcad"]))},
], setup=VIEWS)
add(list_cases, "etkilenenler: yerleştirmesi olmayanın görünümü yok", [
    {"op": "execute", "input": {"affected": True},
     "result": listed(info("Plan", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7400", "attach", F["plan.kcad"], "loaded",
                           inserts=0, objects=1))},
], setup={**UNPLACED, "settings": VIEWS["settings"]})


def resolved(v):
    """None fields dropped, but a result's `output` (a validation's is null)."""
    if isinstance(v, list):
        return [resolved(i) for i in v]
    if isinstance(v, dict):
        return {k_: resolved(i) for k_, i in v.items() if i is not None or k_ == "output"}
    return v


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build(command, title, note, cases):
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', f'  "command": "{command}",',
           '  "commandVersion": 1,', f'  "title": {compact(title)},', f'  "note": {compact(note)},',
           f'  "setup": {compact(OURS)},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "setup" in c:
            lines.append(f'      "setup": {compact(resolved(c["setup"]))},')
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(resolved(s))}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


NOTE = "Written by scripts/fixtures/xref_command_cases.py from docs/adr/0348 and the exchange reference (exchange_cases.py); do not edit by hand."


def main():
    texts = {
        OUT / "cad.xrefs.edit.json": build("cad.xrefs.edit", "Dış başvuru: ekle, yeniden yükle, yolu değiştir, boşalt, kaldır, bağla", NOTE, edit_cases),
        OUT / "cad.xrefs.list.json": build("cad.xrefs.list", "Dış başvurular: durumları dosyalarından", NOTE, list_cases),
    }
    binaries = {SRC / name: b for name, b in FILES.items()}
    if "--check" in sys.argv[1:]:
        for path, text in texts.items():
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                sys.exit(f"{path.relative_to(ROOT)} güncel değil")
        for path, b in binaries.items():
            if not path.exists() or path.read_bytes() != b:
                sys.exit(f"{path.relative_to(ROOT)} güncel değil")
        print(f"{len(edit_cases)} + {len(list_cases)} durum tutarlı")
        return
    for path, text in texts.items():
        path.write_text(text, encoding="utf-8")
    for path, b in binaries.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b)
    print(f"{len(edit_cases)} + {len(list_cases)} durum yazıldı")


main()
