#!/usr/bin/env python3
"""Çizimler arası alışveriş (docs/adr/0193): the shared cases of its three rules, written from the ADR without KentOS
code, over two drawings in the contract's JSON form (`DocumentSnapshotV2`):

- the selection's drawing (Seçilenleri dosyaya kaydet): the selected objects with the layers, groups, blocks and library
  items they and their blocks use, the settings with the kept layers' states, links to objects left out dropped;
- taking from another drawing (Başka çizimden al): layers by path, text and dimension styles, library items by id with
  their images, blocks by name with what they place and the symbols their objects draw with, layer states by name, the
  project's units and scale; the same names skipped or replaced;
- a drawing as a block (Dosyadan blok ekle): its objects but pictures and tables, its base the lower left of their
  extent, their layers by path (made when missing), their blocks and styles brought in under names the drawing has not;
- a layer with its objects (Kaynaklar's Katman olarak ekle, docs/adr/0199 §7): the layer by path (made when missing,
  with its groups; a met one kept as it is), its objects with the layers their blocks' objects are on, the blocks they
  place by name (a met name is the drawing's own), the styles by name (the missing added), the library items the
  objects and the made layers draw with; links, ties and a table's source dropped (the objects take new ids).

The extent's corner is taken here from the objects whose bounds are plain (lines, paths without arcs, circles,
points); the drawings keep every other object well inside them, which this script checks.

    python3 scripts/fixtures/exchange_cases.py          # write
    python3 scripts/fixtures/exchange_cases.py --check  # compare
"""

import copy
import json
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "exchange" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/exchange_cases.py (docs/adr/0193)"

E, N = 487000.0, 4420000.0


def P(x, y):
    return {"x": E + x, "y": N + y}


def style(color, line="continuous", weight=0.25):
    return {"color": color, "lineType": line, "lineWeight": weight}


def layer(i, name, st, children=None, kind="layer", visible=True, locked=False):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": st, "children": children or []}


def group(i, name, children, st=None):
    return layer(i, name, st or style("fg"), children, "group")


U = lambda n: f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{n:04x}"  # noqa: E731


def square(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


# ── The drawings ──────────────────────────────────────────────────────

THEIRS = {
    "format": "kentos.document",
    "version": 2,
    "name": "Kaynak",
    "settings": {
        "srid": 5256,
        "lengthDecimals": 2,
        "areaDecimals": 1,
        "areaUnit": "donum",
        "angleUnit": "grad",
        "plotScale": 1000.0,
        "workspace": "gis",
        "drawingFont": "arimo",
        "textStyles": [
            {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3101", "name": "Ada no", "font": "arimo", "bold": True, "height": 3.5},
            {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3102", "name": "Yol adı", "font": "overpass", "italic": True, "height": 2.5},
        ],
        "dimensionStyles": [{"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3201", "name": "Kadastro", "height": 2.5, "arrow": "none"}],
        "layerStates": [
            {"id": "durum-k", "name": "Baskı", "nodes": [{"node": "k-kadastro", "visible": True}, {"node": "parsel", "visible": True, "locked": True}, {"node": "k-yol", "visible": False}]},
        ],
    },
    "origin": {"x": E, "y": N},
    "layers": [
        layer("0", "0", style("fg")),
        group("k-kadastro", "Kadastro", [layer("parsel", "Parsel", style("#E5484D", weight=0.5)), layer("k-bina", "Bina", style("#8C9AAA"))]),
        layer("k-yol", "Yol", style("#30A46C", "dashed", 0.35)),
    ],
    "activeLayer": "k-yol",
    "entities": [
        {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Ada": "101"}, "symbol": "sym-parsel", "pts": square(0, 0, 40, 30)},
        {"kind": "polygon", "id": 2, "layerId": "k-bina", "attrs": {}, "pts": square(10, 10, 20, 20)},
        {"kind": "line", "id": 3, "layerId": "k-yol", "attrs": {}, "a": P(-10, -5), "b": P(60, -5)},
        {"kind": "text", "id": 4, "layerId": "parsel", "attrs": {}, "p": P(25, 22), "text": "101", "height": 2.0, "rotation": 0.0,
         "textStyle": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3101", "labelOf": U(0x2001), "labelScale": 1000.0},
        {"kind": "insert", "id": 5, "layerId": "k-yol", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "p": P(30, 5), "scale": 1.0, "rotation": 0.0},
        {"kind": "hatch", "id": 6, "layerId": "k-bina", "attrs": {}, "ring": square(10, 10, 20, 20), "pattern": {"type": "solid", "angle": 0.0, "spacing": 1.0},
         "assoc": {"outer": U(0x2002), "seed": P(15, 15)}},
        {"kind": "image", "id": 7, "layerId": "parsel", "attrs": {}, "p": P(22, 2), "width": 8.0, "height": 6.0, "rotation": 0.0, "asset": "img-k-foto"},
        {"kind": "table", "id": 8, "layerId": "parsel", "attrs": {}, "p": P(2, 28), "rotation": 0.0, "height": 1.25, "rows": [2.5], "columns": [8.0], "cells": [["A"]],
         "source": {"kind": "areas", "objects": [U(0x2001), U(0x2002)]}},
        {"kind": "dimension", "id": 9, "layerId": "parsel", "attrs": {}, "a": P(5, 25), "b": P(15, 25), "offset": 1.0, "height": 1.0,
         "dimStyle": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3201"},
        {"kind": "circle", "id": 10, "layerId": "0", "attrs": {}, "c": P(50, 20), "r": 4.0},
    ],
    "uids": [U(0x2001 + i) for i in range(10)],
    "styles": {
        "items": [
            {"kind": "symbol", "id": "sym-parsel", "name": "Parsel dolgusu", "path": ["Semboller"], "geometry": "fill", "layers": [{"type": "imageFill", "asset": "img-k-doku"}]},
            {"kind": "symbol", "id": "sym-agac", "name": "Ağaç", "path": ["Semboller"], "geometry": "marker", "layers": [{"type": "shape", "shape": "circle"}]},
            {"kind": "asset", "id": "img-k-doku", "name": "Doku", "path": ["Görüntülerim"], "format": "png", "data": "data:image/png;base64,iVBORw0KGgo=", "width": 4, "height": 4},
            {"kind": "asset", "id": "img-k-foto", "name": "Foto", "path": ["Resimler"], "format": "png", "data": "data:image/png;base64,iVBORw0KGgoAAA=", "width": 8, "height": 6},
            {"kind": "template", "id": "tpl-k", "name": "Ağaç şablonu", "path": ["Şablonlar"], "tool": "point", "symbol": "sym-agac"},
        ],
        "categories": [],
    },
    "blocks": [
        {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "name": "Direk", "base": {"x": 0.0, "y": 0.0},
         "entities": [{"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 0.0, "y": 3.0}},
                      {"kind": "circle", "id": 2, "layerId": "0", "attrs": {}, "c": {"x": 0.0, "y": 3.0}, "r": 0.5}]},
        {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "name": "Lamba", "base": {"x": 0.0, "y": 0.0},
         "entities": [{"kind": "insert", "id": 1, "layerId": "k-yol", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "p": {"x": 0.0, "y": 0.0}, "scale": 1.0, "rotation": 0.0},
                      {"kind": "text", "id": 2, "layerId": "k-yol", "attrs": {}, "p": {"x": 1.0, "y": 3.0}, "text": "L", "height": 0.5, "rotation": 0.0,
                       "textStyle": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3102"}]},
        {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4003", "name": "Ağaç", "base": {"x": 0.0, "y": 0.0},
         "entities": [{"kind": "circle", "id": 1, "layerId": "k-bina", "attrs": {}, "symbol": "sym-agac", "c": {"x": 0.0, "y": 0.0}, "r": 1.0}]},
    ],
}

OURS = {
    "format": "kentos.document",
    "version": 2,
    "name": "Hedef",
    "settings": {
        "srid": 5256,
        "lengthDecimals": 3,
        "areaDecimals": 2,
        "areaUnit": "m2",
        "angleUnit": "deg",
        "plotScale": 500.0,
        "workspace": "cad",
        "drawingFont": "barlow",
        "textStyles": [{"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5101", "name": "ada NO ", "font": "barlow", "height": 2.5}],
        "layerStates": [{"id": "durum-k", "name": "Eski", "nodes": [{"node": "h-parsel", "visible": False}]}],
    },
    "origin": {"x": E, "y": N},
    "layers": [
        layer("0", "0", style("fg")),
        layer("parsel", "Eski parsel", style("#3E63DD")),
        group("h-kad", "kadastro", [layer("h-parsel", "PARSEL", style("#3E63DD"))]),
    ],
    "activeLayer": "0",
    "entities": [
        {"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": P(100, 0), "b": P(110, 0)},
        {"kind": "insert", "id": 2, "layerId": "0", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001", "p": P(100, 10), "scale": 1.0, "rotation": 0.0},
    ],
    "uids": [U(0x7001), U(0x7002)],
    "styles": {
        "items": [{"kind": "symbol", "id": "sym-agac", "name": "Ağaç (bizim)", "path": ["Semboller"], "geometry": "marker", "layers": [{"type": "shape", "shape": "star"}]}],
        "categories": [],
    },
    "blocks": [
        {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001", "name": "DİREK", "base": {"x": 0.0, "y": 0.0},
         "entities": [{"kind": "point", "id": 1, "layerId": "0", "attrs": {}, "p": {"x": 0.0, "y": 0.0}}]},
    ],
}


# ── Names and paths ───────────────────────────────────────────────────

def fold(name):
    """A name as names are compared: Turkish letters folded, case aside, trimmed (the layers', styles' and states')."""
    s = name.strip().replace("İ", "i").replace("I", "ı").lower()
    return "".join(c for c in unicodedata.normalize("NFD", s) if unicodedata.category(c) != "Mn").replace("ı", "i")


def block_key(name):
    """A block's name as blocks compare them (docs/adr/0144): each letter lowercased, Turkish I (I → ı, İ → i)."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def walk(nodes, above=()):
    for n in nodes:
        here = above + (n["name"],)
        yield n, here
        yield from walk(n["children"], here)


def path_of(tree):
    return {n["id"]: here for n, here in walk(tree)}


def all_ids(tree):
    return {n["id"] for n, _ in walk(tree)}


def free_id(i, taken):
    out, k = i, 2
    while out in taken:
        out, k = f"{i}-{k}", k + 1
    taken.add(out)
    return out


def unique_names(taken, incoming):
    keys = {block_key(t) for t in taken}
    out = []
    for name in incoming:
        chosen, k = name, 2
        while block_key(chosen) in keys:
            chosen, k = f"{name} ({k})", k + 1
        keys.add(block_key(chosen))
        out.append(chosen)
    return out


def strings_at(value, key, out):
    if isinstance(value, dict):
        for k, v in value.items():
            if k == key and isinstance(v, str):
                out.add(v)
            else:
                strings_at(v, key, out)
    elif isinstance(value, list):
        for v in value:
            strings_at(v, key, out)
    return out


# ── 1. The selection's drawing ────────────────────────────────────────

def placed_blocks(entities, by_id):
    """The definitions these objects place, and those they place in turn."""
    out, todo = set(), [e["block"] for e in entities if e["kind"] == "insert"]
    while todo:
        b = todo.pop()
        if b in out or b not in by_id:
            continue
        out.add(b)
        todo += [e["block"] for e in by_id[b]["entities"] if e["kind"] == "insert"]
    return out


def selection(doc, uids):
    doc = copy.deepcopy(doc)
    kept_uids = [u for u in doc["uids"] if u in set(uids)]
    keep = set(kept_uids)
    entities = [e for e, u in zip(doc["entities"], doc["uids"]) if u in keep]
    by_id = {b["id"]: b for b in doc.get("blocks", [])}
    blocks = placed_blocks(entities, by_id)
    kept_blocks = [b for b in doc.get("blocks", []) if b["id"] in blocks]
    used_layers = {e["layerId"] for e in entities} | {e["layerId"] for b in kept_blocks for e in b["entities"]}

    def prune(nodes):
        out = []
        for n in nodes:
            kids = prune(n["children"])
            if (n["type"] == "layer" and n["id"] in used_layers) or (n["type"] == "group" and kids):
                out.append({**n, "children": kids})
        return out

    tree = prune(doc["layers"])
    kept_nodes = all_ids(tree)
    leaves = [n["id"] for n, _ in walk(tree) if n["type"] == "layer"]
    active = doc["activeLayer"] if doc["activeLayer"] in leaves else leaves[0]
    out = []
    for i, e in enumerate(entities):
        e = {**e, "id": i + 1}
        if e["kind"] == "text" and "labelOf" in e and e["labelOf"] not in keep:
            e.pop("labelOf")
            e.pop("labelScale", None)
        if e["kind"] == "hatch" and "assoc" in e:
            a = e["assoc"]
            if not all(u in keep for u in [a["outer"], *a.get("islands", []), *a.get("cutouts", [])]):
                e.pop("assoc")
        if e["kind"] == "table" and isinstance(e.get("source"), dict) and "objects" in e["source"]:
            objs = [u for u in e["source"]["objects"] if u in keep]
            if objs:
                e["source"] = {**e["source"], "objects": objs}
            else:
                e.pop("source")
        out.append(e)
    # The library: what the kept objects, blocks' objects and layers use.
    items = doc["styles"]["items"]
    symbols = {e["symbol"] for e in out if "symbol" in e} | {e["symbol"] for b in kept_blocks for e in b["entities"] if "symbol" in e}
    layer_styles = [n["style"] for n, _ in walk(tree)]
    for st in layer_styles:
        strings_at(st, "ref", symbols)
    assets = {e["asset"] for e in out if e["kind"] == "image" and "asset" in e}
    for it in items:
        if it["kind"] == "symbol" and it["id"] in symbols:
            strings_at(it, "asset", assets)
    for st in layer_styles:
        strings_at(st, "asset", assets)
    kept_items = [it for it in items if (it["kind"] == "symbol" and it["id"] in symbols) or (it["kind"] == "asset" and it["id"] in assets)]
    settings = copy.deepcopy(doc["settings"])
    states = []
    for st in settings.get("layerStates", []):
        nodes = [n for n in st["nodes"] if n["node"] in kept_nodes]
        if nodes:
            states.append({**st, "nodes": nodes})
    if states:
        settings["layerStates"] = states
    else:
        settings.pop("layerStates", None)
    result = {
        "format": doc["format"],
        "version": 2,
        "name": "Seçim",
        "settings": settings,
        "origin": doc["origin"],
        "layers": tree,
        "activeLayer": active,
        "entities": out,
        "uids": kept_uids,
        "styles": {"items": kept_items, "categories": doc["styles"]["categories"]},
    }
    if kept_blocks:
        result["blocks"] = kept_blocks
    return result


# ── 2. Taking from another drawing ────────────────────────────────────

TAKEN_SETTINGS = ("lengthDecimals", "areaDecimals", "areaUnit", "angleUnit", "plotScale", "drawingFont", "drawingUnit", "survey")


def find_path(tree, names):
    """Our node at these names (folded), or None; the last node found and how deep."""
    nodes, found = tree, None
    for name in names:
        hit = next((n for n in nodes if fold(n["name"]) == fold(name)), None)
        if hit is None:
            return None
        found, nodes = hit, hit["children"]
    return found


def take_layers(ours, theirs, paths, same, taken):
    their = path_of(theirs["layers"])
    by_path = {here: n for n, here in walk(theirs["layers"])}
    for n, here in walk(theirs["layers"]):
        if n["type"] != "layer" or " / ".join(here) not in paths:
            continue
        parent_list, kind_ok = ours["layers"], True
        for depth in range(len(here)):
            names = here[: depth + 1]
            source = by_path[names]
            hit = next((m for m in parent_list if fold(m["name"]) == fold(names[-1])), None)
            last = depth == len(here) - 1
            if hit is None:
                made = {**copy.deepcopy(source), "id": free_id(source["id"], taken), "children": []}
                parent_list.append(made)
                parent_list = made["children"]
                continue
            if hit["type"] != source["type"]:
                kind_ok = False
                break
            if last and same == "replace":
                hit["style"] = copy.deepcopy(source["style"])
            parent_list = hit["children"]
        del kind_ok
    return their


def our_node_for(ours, their_paths, their_id, layers_only=True):
    """Our node at the path their node has; a layer only, unless a state names groups too."""
    here = their_paths.get(their_id)
    if here is None:
        return None
    hit = find_path(ours["layers"], here)
    return hit["id"] if hit and (hit["type"] == "layer" or not layers_only) else None


def take_styles(ours, theirs, key, names, same):
    mine = ours["settings"].setdefault(key, [])
    taken_ids = {s["id"] for s in mine}
    for s in theirs["settings"].get(key, []):
        if s["name"] not in names:
            continue
        hit = next((m for m in mine if fold(m["name"]) == fold(s["name"])), None)
        if hit is not None:
            if same == "replace":
                i = mine.index(hit)
                mine[i] = {**copy.deepcopy(s), "id": hit["id"]}
            continue
        assert s["id"] not in taken_ids, "the cases give styles ids the drawing has not"
        mine.append(copy.deepcopy(s))
    if not mine:
        ours["settings"].pop(key)


def take_items(ours, theirs, ids, same):
    mine = ours["styles"]["items"]
    by_id = {it["id"]: it for it in theirs["styles"]["items"]}
    wanted = []
    for i in ids:
        if i in by_id and i not in wanted:
            wanted.append(i)
            if by_id[i]["kind"] == "symbol":
                for a in sorted(strings_at(by_id[i], "asset", set())):
                    if a in by_id and a not in wanted:
                        wanted.append(a)
    order = [it["id"] for it in theirs["styles"]["items"]]
    for i in sorted(wanted, key=order.index):
        hit = next((k for k, m in enumerate(mine) if m["id"] == i), None)
        if hit is not None:
            if same == "replace":
                mine[hit] = copy.deepcopy(by_id[i])
            continue
        mine.append(copy.deepcopy(by_id[i]))


def style_ids(ours, theirs):
    """Their text and dimension style ids → ours by name (none when we have no such style)."""
    out = {}
    for key in ("textStyles", "dimensionStyles"):
        for s in theirs["settings"].get(key, []):
            hit = next((m for m in ours["settings"].get(key, []) if fold(m["name"]) == fold(s["name"])), None)
            out[s["id"]] = hit["id"] if hit else None
    return out


def map_object(e, layer_of, block_of, styles):
    e = copy.deepcopy(e)
    e["layerId"] = layer_of(e["layerId"])
    if e["kind"] == "insert":
        e["block"] = block_of[e["block"]]
    for field in ("textStyle", "dimStyle"):
        if field in e:
            to = styles.get(e[field])
            if to is None:
                e.pop(field)
            else:
                e[field] = to
    e.pop("labelOf", None)
    e.pop("labelScale", None)
    e.pop("assoc", None)
    return e


def take_blocks(ours, theirs, names, same, their_paths):
    by_id = {b["id"]: b for b in theirs.get("blocks", [])}
    picked = {b["id"] for b in theirs.get("blocks", []) if b["name"] in names}
    wanted = placed_blocks([{"kind": "insert", "block": b} for b in picked], by_id)
    mine = ours.setdefault("blocks", [])
    block_of = {}
    for b in theirs.get("blocks", []):
        if b["id"] not in wanted:
            continue
        hit = next((m for m in mine if block_key(m["name"]) == block_key(b["name"])), None)
        block_of[b["id"]] = hit["id"] if hit else b["id"]
    # The symbols their objects draw with, when we have none such.
    symbols = []
    for b in theirs.get("blocks", []):
        if b["id"] in wanted:
            symbols += [e["symbol"] for e in b["entities"] if "symbol" in e]
    have = {it["id"] for it in ours["styles"]["items"]}
    take_items(ours, theirs, [s for s in symbols if s not in have], "skip")
    styles = style_ids(ours, theirs)
    layer_of = lambda i: our_node_for(ours, their_paths, i) or ""  # noqa: E731
    for b in theirs.get("blocks", []):
        if b["id"] not in wanted:
            continue
        defined = {**copy.deepcopy(b), "id": block_of[b["id"]], "entities": [map_object(e, layer_of, block_of, styles) for e in b["entities"]]}
        hit = next((k for k, m in enumerate(mine) if block_key(m["name"]) == block_key(b["name"])), None)
        if hit is not None:
            if same == "replace":
                mine[hit] = {**defined, "name": mine[hit]["name"]}
            continue
        mine.append(defined)
    if not mine:
        ours.pop("blocks")


def take_states(ours, theirs, names, same, their_paths):
    mine = ours["settings"].setdefault("layerStates", [])
    taken = {s["id"] for s in mine}
    for s in theirs["settings"].get("layerStates", []):
        if s["name"] not in names:
            continue
        nodes = []
        for n in s["nodes"]:
            to = our_node_for(ours, their_paths, n["node"], layers_only=False)
            if to is not None:
                nodes.append({**copy.deepcopy(n), "node": to})
        hit = next((k for k, m in enumerate(mine) if fold(m["name"]) == fold(s["name"])), None)
        if hit is not None:
            if same == "replace":
                mine[hit] = {**mine[hit], "nodes": nodes}
            continue
        mine.append({**copy.deepcopy(s), "id": free_id(s["id"], taken), "nodes": nodes})
    if not mine:
        ours["settings"].pop("layerStates")


def take(ours, theirs, picks, same):
    ours = copy.deepcopy(ours)
    if picks.get("settings"):
        for key in TAKEN_SETTINGS:
            if key in theirs["settings"]:
                ours["settings"][key] = copy.deepcopy(theirs["settings"][key])
            else:
                ours["settings"].pop(key, None)
    taken = all_ids(ours["layers"])
    their_paths = take_layers(ours, theirs, set(picks.get("layers", [])), same, taken)
    take_styles(ours, theirs, "textStyles", set(picks.get("textStyles", [])), same)
    take_styles(ours, theirs, "dimensionStyles", set(picks.get("dimensionStyles", [])), same)
    take_items(ours, theirs, picks.get("library", []), same)
    take_blocks(ours, theirs, set(picks.get("blocks", [])), same, their_paths)
    take_states(ours, theirs, set(picks.get("layerStates", [])), same, their_paths)
    return ours


# ── 3. A drawing as a block ───────────────────────────────────────────

def plain_bounds(e):
    """The bounds of an object whose bounds are plain; None for any other."""
    k = e["kind"]
    if k == "line":
        pts = [e["a"], e["b"]]
    elif k in ("polygon", "polyline") and not any(e.get("bulges", [])):
        pts = e["pts"]
    elif k == "point":
        pts = [e["p"]]
    elif k == "circle":
        c, r = e["c"], e["r"]
        pts = [{"x": c["x"] - r, "y": c["y"] - r}, {"x": c["x"] + r, "y": c["y"] + r}]
    else:
        return None
    return min(p["x"] for p in pts), min(p["y"] for p in pts)


def anchor_points(e):
    k = e["kind"]
    if k in ("text", "insert"):
        return [e["p"]]
    if k == "hatch":
        return e["ring"]
    if k == "dimension":
        return [e["a"], e["b"]]
    return []


def file_block(ours, theirs, file_name):
    ours = copy.deepcopy(ours)
    kept = [e for e in theirs["entities"] if e["kind"] not in ("image", "table")]
    left = {"images": sum(e["kind"] == "image" for e in theirs["entities"]), "tables": sum(e["kind"] == "table" for e in theirs["entities"])}
    corners = [b for b in (plain_bounds(e) for e in kept) if b is not None]
    base = {"x": min(c[0] for c in corners), "y": min(c[1] for c in corners)}
    for e in kept:
        for p in anchor_points(e):
            assert p["x"] > base["x"] + 1 and p["y"] > base["y"] + 1, "the cases keep the other objects inside"
    # Layers by path, the missing made.
    their_paths = path_of(theirs["layers"])
    taken = all_ids(ours["layers"])
    needed = []
    for e in kept:
        if e["layerId"] not in needed:
            needed.append(e["layerId"])
    by_id = {b["id"]: b for b in theirs.get("blocks", [])}
    nested = placed_blocks(kept, by_id)
    for b in theirs.get("blocks", []):
        if b["id"] in nested:
            for e in b["entities"]:
                if e["layerId"] not in needed:
                    needed.append(e["layerId"])
    wanted_paths = {" / ".join(their_paths[i]) for i in needed if i in their_paths}
    take_layers(ours, theirs, wanted_paths, "skip", taken)
    # Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
    styles = {}
    for key, field in (("textStyles", "textStyle"), ("dimensionStyles", "dimStyle")):
        used = {e[field] for e in kept if field in e} | {e[field] for b in theirs.get("blocks", []) if b["id"] in nested for e in b["entities"] if field in e}
        mine = ours["settings"].setdefault(key, [])
        for s in theirs["settings"].get(key, []):
            if s["id"] not in used:
                continue
            hit = next((m for m in mine if fold(m["name"]) == fold(s["name"])), None)
            if hit is None:
                mine.append(copy.deepcopy(s))
                styles[s["id"]] = s["id"]
            else:
                styles[s["id"]] = hit["id"]
        if not mine:
            ours["settings"].pop(key)
    # The library items the objects draw with.
    have = {it["id"] for it in ours["styles"]["items"]}
    symbols = [e["symbol"] for e in kept if "symbol" in e] + [e["symbol"] for b in theirs.get("blocks", []) if b["id"] in nested for e in b["entities"] if "symbol" in e]
    take_items(ours, theirs, [s for s in symbols if s not in have], "skip")
    # The blocks: the nested ones first (their order), then the drawing's own.
    mine = ours.setdefault("blocks", [])
    order = [b for b in theirs.get("blocks", []) if b["id"] in nested]
    names = unique_names([m["name"] for m in mine], [b["name"] for b in order] + [file_name])
    block_of = {b["id"]: b["id"] for b in order}
    layer_of = lambda i: our_node_for(ours, their_paths, i) or ""  # noqa: E731
    for b, name in zip(order, names):
        mine.append({**copy.deepcopy(b), "name": name, "entities": [map_object(e, layer_of, block_of, styles) for e in b["entities"]]})
    entities = []
    for i, e in enumerate(kept):
        m = map_object(e, layer_of, block_of, styles)
        m["id"] = i + 1
        entities.append(m)
    mine.append({"id": "$new", "name": names[-1], "base": base, "entities": entities})
    return ours, left


# ── 4. A layer with its objects ───────────────────────────────────────

def layer_take(ours, theirs, path):
    """Their layer at `path` with its objects into `ours`: the new drawing (its objects as they were) and the objects
    to add, numbered from 1; None when `path` is not a layer of theirs or our node at it is not a layer."""
    ours = copy.deepcopy(ours)
    target = next((n for n, here in walk(theirs["layers"]) if n["type"] == "layer" and " / ".join(here) == path), None)
    if target is None:
        return None
    objects = [e for e in theirs["entities"] if e["layerId"] == target["id"]]
    by_id = {b["id"]: b for b in theirs.get("blocks", [])}
    nested = placed_blocks(objects, by_id)
    nested_blocks = [b for b in theirs.get("blocks", []) if b["id"] in nested]
    pieces = objects + [e for b in nested_blocks for e in b["entities"]]
    their_paths = path_of(theirs["layers"])
    wanted = {path} | {" / ".join(their_paths[e["layerId"]]) for e in pieces if e["layerId"] in their_paths}
    before = all_ids(ours["layers"])
    take_layers(ours, theirs, wanted, "skip", set(before))
    if our_node_for(ours, their_paths, target["id"]) is None:
        return None
    made_styles = [n["style"] for n, _ in walk(ours["layers"]) if n["id"] not in before]
    # Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
    styles = {}
    for key, field in (("textStyles", "textStyle"), ("dimensionStyles", "dimStyle")):
        used = {e[field] for e in pieces if field in e}
        mine = ours["settings"].setdefault(key, [])
        for s in theirs["settings"].get(key, []):
            if s["id"] not in used:
                continue
            hit = next((m for m in mine if fold(m["name"]) == fold(s["name"])), None)
            if hit is None:
                assert s["id"] not in {m["id"] for m in mine}, "the cases give styles ids the drawing has not"
                mine.append(copy.deepcopy(s))
                styles[s["id"]] = s["id"]
            else:
                styles[s["id"]] = hit["id"]
        if not mine:
            ours["settings"].pop(key)
    # The library items the objects, their pictures and the made layers draw with, when we have none such.
    have = {it["id"] for it in ours["styles"]["items"]}
    items = []
    for e in objects:
        if "symbol" in e:
            items.append(e["symbol"])
        if e["kind"] == "image" and "asset" in e:
            items.append(e["asset"])
    for st in made_styles:
        items += sorted(strings_at(st, "ref", set())) + sorted(strings_at(st, "asset", set()))
    take_items(ours, theirs, [i for i in items if i not in have], "skip")
    # The blocks by name: a met one is ours, the others come with what they place.
    take_blocks(ours, theirs, {b["name"] for b in nested_blocks}, "skip", their_paths)
    block_of = {b["id"]: next(m["id"] for m in ours.get("blocks", []) if block_key(m["name"]) == block_key(b["name"])) for b in nested_blocks}
    layer_of = lambda i: our_node_for(ours, their_paths, i) or ""  # noqa: E731
    out = []
    for i, e in enumerate(objects):
        m = map_object(e, layer_of, block_of, styles)
        m.pop("source", None)
        m["id"] = i + 1
        out.append(m)
    return {"drawing": ours, "objects": out}


# The source drawing with a layer whose look draws with a library symbol (its picture too).
THEIRS_LOOK = copy.deepcopy(THEIRS)
THEIRS_LOOK["name"] = "Kaynak (görünüş)"
THEIRS_LOOK["layers"][1]["children"][1]["style"]["renderer"] = {"type": "single", "symbol": {"ref": "sym-parsel"}}


# ── The cases ─────────────────────────────────────────────────────────

def build():
    picks = {
        "layers": ["Kadastro / Parsel", "Kadastro / Bina", "Yol"],
        "blocks": ["Lamba", "Ağaç"],
        "textStyles": ["Ada no", "Yol adı"],
        "dimensionStyles": ["Kadastro"],
        "library": ["sym-parsel", "sym-agac", "tpl-k"],
        "layerStates": ["Baskı"],
    }
    file_drawing, left = file_block(OURS, THEIRS, "Kaynak")
    return {
        "format": "kentos.exchange-cases",
        "version": 1,
        "source": SOURCE,
        "drawings": {"ours": OURS, "theirs": THEIRS, "theirsLook": THEIRS_LOOK},
        "selections": [
            {"name": "parsel, bağlı yazısı, lamba (iç içe Direk), ilişkisi kaydedilmeyen nesneye taramanın, tablo ve resim",
             "from": "theirs", "uids": [U(0x2001), U(0x2004), U(0x2005), U(0x2006), U(0x2007), U(0x2008)],
             "expect": selection(THEIRS, [U(0x2001), U(0x2004), U(0x2005), U(0x2006), U(0x2007), U(0x2008)])},
            {"name": "yalnız yol çizgisi: tek katman, etkin katman o",
             "from": "theirs", "uids": [U(0x2003)], "expect": selection(THEIRS, [U(0x2003)])},
            {"name": "bağlı yazı nesnesiyle, ilişkili tarama nesnesiyle: bağlar kalır",
             "from": "theirs", "uids": [U(0x2002), U(0x2006), U(0x2001), U(0x2004)],
             "expect": selection(THEIRS, [U(0x2002), U(0x2006), U(0x2001), U(0x2004)])},
        ],
        "takes": [
            {"name": "aynı adlılar atlanır", "into": "ours", "from": "theirs", "picks": picks, "same": "skip", "expect": take(OURS, THEIRS, picks, "skip")},
            {"name": "aynı adlılar değiştirilir, proje ayarları da", "into": "ours", "from": "theirs", "picks": {**picks, "settings": True}, "same": "replace",
             "expect": take(OURS, THEIRS, {**picks, "settings": True}, "replace")},
            {"name": "yalnız proje ayarları", "into": "ours", "from": "theirs", "picks": {"settings": True}, "same": "skip", "expect": take(OURS, THEIRS, {"settings": True}, "skip")},
        ],
        "files": [
            {"name": "kaynak çizim blok olur: resim ve tablo kalır, katmanlar yoluyla açılır, Direk'in adı sayı alır",
             "into": "ours", "from": "theirs", "file": "Kaynak", "expect": file_drawing, "left": left},
        ],
        "layers": [
            {"name": "bizde de olan katman: nesneler bizimkine, simgesi ve resmi kitaplığa, stil adıyla eşlenir ya da eklenir, bağ ve tablonun kaynağı düşer",
             "into": "ours", "from": "theirs", "path": "Kadastro / Parsel", "expect": layer_take(OURS, THEIRS, "Kadastro / Parsel")},
            {"name": "bizde olmayan katman açılır; lamba bloğu iç içe Direk'iyle, aynı adlı Direk bizimki",
             "into": "ours", "from": "theirs", "path": "Yol", "expect": layer_take(OURS, THEIRS, "Yol")},
            {"name": "açılan katmanın görünüşünün simgesi ve resmi de gelir, taramanın ilişkisi düşer",
             "into": "ours", "from": "theirsLook", "path": "Kadastro / Bina", "expect": layer_take(OURS, THEIRS_LOOK, "Kadastro / Bina")},
            {"name": "grup katman değildir", "into": "ours", "from": "theirs", "path": "Kadastro", "expect": layer_take(OURS, THEIRS, "Kadastro")},
            {"name": "olmayan yol", "into": "ours", "from": "theirs", "path": "Yok", "expect": layer_take(OURS, THEIRS, "Yok")},
        ],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"alışveriş durumları tutarlı: {OUT.relative_to(ROOT)}")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
