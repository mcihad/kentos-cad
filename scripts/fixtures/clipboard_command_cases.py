"""Panoya kopyala and Yapıştır: shared cases of cad.entities.copy and cad.entities.paste (docs/adr/0309).

    python3 scripts/fixtures/clipboard_command_cases.py           # writes the files
    python3 scripts/fixtures/clipboard_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.copy.json and cad.entities.paste.json. Without KentOS code: the copy is the
selection's drawing and the paste the exchange's layer and block rules, as scripts/fixtures/exchange_cases.py writes
them from ADR 0193 and 0199 §7 (that reference is used here, not KentOS):

- a copy holds the objects in the drawing's order (numbered 1 …), their persistent ids, the layer tree pruned to
  their layers and the groups above them, the blocks they place (nested too), the text and dimension styles they
  name (not the others), the library items they use, the source's unit and the base point (given, or the lower
  left corner of their boxes);
- a paste takes each of the copy's layers with its objects, in the order the objects come: the layer by path (a met
  path is ours, a missing one made), a met block name is ours (its definition used), the others added with what
  they place, styles by name (the missing added); links that leave the copy drop; the objects moved so the copy's
  base point lands at `at`, written in the copy's order as one step “Yapıştır”;
- as a block: the copy's objects one block named (a free name), its base the lower left corner of their boxes, the
  nested blocks under free names, placed so the copy's base point lands at `at` on the active layer;
- a locked layer an object would go to refuses, a hidden one is said; another unit is said, sizes kept;
- `conflicts: rename` (docs/adr/0351 §4): a block or a style of the copy whose name this drawing has (blocks by the
  block key, styles folded) comes under the first free “Ad (2)”, “Ad (3)” … (free of this drawing's and the copy's
  names), said in `report.renamed` and a `names_renamed` warning before the unit's; a renamed block takes a new id
  its inserts follow (here `$blockOf:Ad (2)`).
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import exchange_cases as ex  # noqa: E402  (KentOS-free: written from the ADRs)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Yapıştır"


def P(x, y):
    return {"x": x, "y": y}


def node(i, name, kind="layer", children=None, locked=False, visible=True, color="fg"):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {**STYLE, "color": color}, "children": children or []}


def base(i, layer, **more):
    return {"id": i, "layerId": layer, "attrs": {}, **more}


# ── The source drawing ─────────────────────────────────────────────────────────────────────────────────────────────

KAPAK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11"
ROGAR = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12"
TS_TITLE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101"
TS_OLD = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102"
DS_PLAN = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001"

A_LAYERS = [
    node("cizim", "Çizim"),
    node("kadastro", "Kadastro", "group", [node("parsel", "Parsel", color="#E5484D"), node("bina", "Bina")]),
    node("kilitli", "Kilitli", locked=True),
    node("gizli", "Gizli", visible=False),
]
A_BLOCKS = [
    {"id": KAPAK, "name": "Kapak", "base": P(0, 0),
     "entities": [{"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": P(-0.5, 0), "b": P(0.5, 0)}]},
    {"id": ROGAR, "name": "Rögar", "base": P(0, 0),
     "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": P(0, 0), "r": 0.6},
                  {"kind": "insert", "id": 2, "layerId": "0", "attrs": {}, "block": KAPAK, "p": P(0, 0), "scale": 1,
                   "rotation": 0}]},
]
A_ENTITIES = [
    {"kind": "line", **base(1, "cizim"), "a": P(0, 0), "b": P(10, 0)},
    {"kind": "polygon", **base(2, "parsel"), "attrs": {"No": "101"},
     "pts": [P(0, 10), P(10, 10), P(10, 20), P(0, 20)]},
    {"kind": "text", **base(3, "bina"), "p": P(2, 22), "text": "Bina A", "height": 1, "rotation": 0,
     "textStyle": TS_TITLE},
    {"kind": "insert", **base(4, "cizim"), "block": ROGAR, "p": P(30, 0), "scale": 1, "rotation": 0},
    {"kind": "dimension", **base(5, "cizim"), "a": P(0, -5), "b": P(10, -5), "offset": 1, "height": 0.25,
     "dimStyle": DS_PLAN},
    {"kind": "line", **base(6, "kilitli"), "a": P(0, 30), "b": P(10, 30)},
    {"kind": "line", **base(7, "gizli"), "a": P(0, 35), "b": P(10, 35)},
]
A_TEXT_STYLES = [{"id": TS_TITLE, "name": "Başlık", "font": "arimo", "bold": True},
                 {"id": TS_OLD, "name": "Eski", "font": "overpass"}]
A_DIM_STYLES = [{"id": DS_PLAN, "name": "Mimari", "height": 3.5}]


def settings(text=(), dims=(), unit=None):
    s = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
         "workspace": "cad"}
    if unit:
        s["drawingUnit"] = unit
    if text:
        s["textStyles"] = list(text)
    if dims:
        s["dimensionStyles"] = list(dims)
    return s


def drawing(name, layers, entities, blocks, sett, active="cizim"):
    d = {"format": "kentos.document", "version": 1, "name": name, "settings": sett, "origin": P(0, 0),
         "layers": layers, "activeLayer": active, "entities": entities, "styles": {"items": [], "categories": []}}
    if blocks:
        d["blocks"] = blocks
    return d


A = drawing("Kaynak", A_LAYERS, A_ENTITIES, A_BLOCKS, settings(A_TEXT_STYLES, A_DIM_STYLES))

# ── The other drawing: millimetres, its own Rögar, a Kadastro group without Parsel, a “Yapı” group ──────────────────

RB = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e21"
TSB = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201"
B_LAYERS = [
    node("cizim", "Çizim"),
    node("kad", "KADASTRO", "group", [node("b2", "bina")]),
    node("yapi", "Yapı", "group", [node("duvar", "Duvar")]),
    node("kl", "Kilitli", locked=True),
    node("gz", "Gizli", visible=False),
]
B_BLOCKS = [{"id": RB, "name": "rögar", "base": P(0, 0),
             "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": P(0, 0), "r": 0.5}]}]
B_ENTITIES = [{"kind": "line", **base(1, "cizim"), "a": P(0, 100), "b": P(5, 100)}]
B_TEXT_STYLES = [{"id": TSB, "name": "başlık", "font": "barlow"}]
B = drawing("Hedef", B_LAYERS, B_ENTITIES, B_BLOCKS, settings(B_TEXT_STYLES, unit="mm"))


def v2(doc):
    """A drawing as the exchange's rules read it: its objects' persistent ids the runner's placeholders."""
    d = copy.deepcopy(doc)
    d["uids"] = [U(e["id"]) for e in d["entities"]]
    d.setdefault("blocks", [])
    return d


# ── Kopyala ────────────────────────────────────────────────────────────────────────────────────────────────────────

def used_styles(objects, blocks, key, field, styles):
    used = {e[field] for e in objects if field in e} | {e[field] for b in blocks for e in b["entities"] if field in e}
    return [s for s in styles if s["id"] in used]


def default_base(objects):
    corners = [b for b in (ex.plain_bounds(e) for e in objects) if b is not None]
    assert corners, "a default base needs a plain object"
    x, y = min(c[0] for c in corners), min(c[1] for c in corners)
    for e in objects:
        for p in ex.anchor_points(e):
            assert p["x"] > x + 1 and p["y"] > y + 1, "the cases keep the other objects inside"
    return P(x, y)


def copy_of(doc, ids, at=None):
    """The copy of the objects `ids` of `doc` (the setup's numbering)."""
    d = v2(doc)
    sel = ex.selection(d, [U(i) for i in ids])
    blocks = sel.get("blocks", [])
    clip = {"objects": sel["entities"], "uids": sel["uids"], "layers": sel["layers"]}
    if blocks:
        clip["blocks"] = blocks
    text = used_styles(sel["entities"], blocks, "textStyles", "textStyle", d["settings"].get("textStyles", []))
    dims = used_styles(sel["entities"], blocks, "dimensionStyles", "dimStyle",
                       d["settings"].get("dimensionStyles", []))
    if text:
        clip["textStyles"] = text
    if dims:
        clip["dimensionStyles"] = dims
    if sel["styles"]["items"]:
        clip["library"] = sel["styles"]
    if "drawingUnit" in d["settings"]:
        clip["unit"] = d["settings"]["drawingUnit"]
    clip["base"] = at if at is not None else default_base(sel["entities"])
    return clip


def copy_step(ids, at=None, op="execute", uids=None):
    inp = {"uids": uids if uids is not None else [U(i) for i in ids]}
    if at is not None:
        inp["base"] = at
    return {"op": op, "input": inp, "result": {"status": "completed", "warnings": [], "output": copy_of(A, ids, at)},
            "expect": {"revision": "same", "canUndo": False}}


def refused(inp, code, message, path, extra=None):
    step = {"op": "execute", "input": inp,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": {"revision": "same"}}
    if extra:
        step.update(extra)
    return step


NOT_UID = ("“on iki” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
           "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.")
GONE = "0192f5a2-0000-7000-8000-00000000ffff"
NOT_FOUND = (f"“{GONE}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin "
             "kimliğini verin.")


def copy_cases():
    c = lambda name, *steps: {"name": name, "steps": list(steps)}  # noqa: E731
    return [
        c("Bir çizgi: kendisi, katmanı, taban noktası kutusunun sol alt köşesi; çizim değişmez", copy_step([1])),
        c("Bir alan: katmanı üstündeki grubuyla, grubun öbür katmanı olmadan", copy_step([2])),
        c("Bir yazı: adını andığı yazı stili, öbürü değil", copy_step([3], at=P(0, 0))),
        c("Bir blok yerleştirmesi: bloğu ve onun yerleştirdiği blok", copy_step([4], at=P(30, 0))),
        c("Bir ölçü: ölçü stili", copy_step([5], at=P(0, -6))),
        c("Hepsi birlikte: katmanlar, bloklar, stiller", copy_step([1, 2, 3, 4, 5], at=P(0, -6))),
        c("Verilen taban noktası", copy_step([1, 2], at=P(5, 5))),
        c("Verilme sırası değil, çizimin sırası", copy_step([1, 2], uids=[U(2), U(1)])),
        c("Aynı kimlik iki kez: bir kez", copy_step([1], uids=[U(1), U(1)])),
        c("Kilitli katmandaki nesne kopyalanır: kopyalamak çizimi değiştirmez", copy_step([6])),
        c("Gizli katmandaki nesne de", copy_step([7])),
        c("Çizgi ve blok: taban çizginin köşesi, blok içeride", copy_step([1, 4], at=P(0, 0))),
        c("Plan aynı kopyayı verir", copy_step([1, 2], op="plan")),
        c("Boş liste: no_entities",
          refused({"uids": []}, "no_entities", "Kopyalanacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.",
                  "uids")),
        c("Kimlik UUID değil: invalid_uid", refused({"uids": ["on iki"]}, "invalid_uid", NOT_UID, "uids[0]")),
        c("Çizimde olmayan nesne: entity_not_found", refused({"uids": [GONE]}, "entity_not_found", NOT_FOUND, "uids[0]")),
        c("Taban noktası sayı değil: not_finite",
          refused({"uids": [U(1)], "base": P(0, 0)}, "not_finite", "Taban noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                  "base.x", {"nonFinite": {"base.x": "NaN"}})),
        c("Doğrulama yazmaz", {"op": "validate", "input": {"uids": [U(1)]},
                               "result": {"status": "completed", "output": None, "warnings": []},
                               "expect": {"revision": "same"}}),
        c("İkinci kimlik çizimde yok: entity_not_found, yeriyle",
          refused({"uids": [U(1), GONE]}, "entity_not_found", NOT_FOUND, "uids[1]")),
        c("Alan ve yazı: iki katman aynı grupta", copy_step([2, 3])),
    ]


# ── Yapıştır ───────────────────────────────────────────────────────────────────────────────────────────────────────

def move(e, dx, dy):
    e = copy.deepcopy(e)
    m = lambda p: P(p["x"] + dx, p["y"] + dy)  # noqa: E731
    for key in ("a", "b", "p", "c"):
        if key in e:
            e[key] = m(e[key])
    if "pts" in e:
        e["pts"] = [m(p) for p in e["pts"]]
    return e


def layer_paths(tree):
    return [" / ".join(here) for _, here in ex.walk(tree)]


def paste_of(target, clip, at=None):
    """What pasting `clip` into `target` makes: the drawing after, the objects (in order) and the report."""
    ours = v2(target)
    theirs = v2(drawing("Pano", clip["layers"], [{**e} for e in clip["objects"]], clip.get("blocks", []),
                        settings(clip.get("textStyles", ()), clip.get("dimensionStyles", ()))))
    theirs["uids"] = clip["uids"]
    theirs["styles"] = clip.get("library", {"items": [], "categories": []})
    their_paths = ex.path_of(theirs["layers"])
    order = []
    for e in clip["objects"]:
        if e["layerId"] not in order:
            order.append(e["layerId"])
    placed = [None] * len(clip["objects"])
    for lid in order:
        made = ex.layer_take(ours, theirs, " / ".join(their_paths[lid]))
        assert made is not None
        ours, objects = made["drawing"], made["objects"]
        slots = [i for i, e in enumerate(clip["objects"]) if e["layerId"] == lid]
        for i, o in zip(slots, objects):
            placed[i] = o
    to = at if at is not None else clip["base"]
    dx, dy = to["x"] - clip["base"]["x"], to["y"] - clip["base"]["y"]
    objects = [move(o, dx, dy) for o in placed]
    before = set(layer_paths(target["layers"]))
    our_blocks = {ex.block_key(b["name"]) for b in target.get("blocks", [])}
    our_styles = {ex.fold(s["name"]) for s in target["settings"].get("textStyles", []) + target["settings"].get("dimensionStyles", [])}
    report = {
        "layers": [p for p in layer_paths(ours["layers"]) if p not in before],
        "blocks": [b["name"] for b in ours.get("blocks", []) if ex.block_key(b["name"]) not in our_blocks],
        "kept": [b["name"] for b in clip.get("blocks", []) if ex.block_key(b["name"]) in our_blocks],
        "styles": [s["name"] for s in ours["settings"].get("textStyles", []) + ours["settings"].get("dimensionStyles", [])
                   if ex.fold(s["name"]) not in our_styles],
    }
    return ours, objects, report


def paste_as_block(target, clip, name, at=None):
    ours = v2(target)
    theirs = v2(drawing("Pano", clip["layers"], [{**e} for e in clip["objects"]], clip.get("blocks", []),
                        settings(clip.get("textStyles", ()), clip.get("dimensionStyles", ()))))
    theirs["uids"] = clip["uids"]
    theirs["styles"] = clip.get("library", {"items": [], "categories": []})
    ours, _left = ex.file_block(ours, theirs, name)
    made = ours["blocks"][-1]
    corner = made["base"]
    to = at if at is not None else clip["base"]
    p = P(to["x"] + corner["x"] - clip["base"]["x"], to["y"] + corner["y"] - clip["base"]["y"])
    insert = {"kind": "insert", "layerId": target["activeLayer"], "attrs": {}, "block": f"$blockOf:{made['name']}",
              "p": p, "scale": 1, "rotation": 0}
    before = set(layer_paths(target["layers"]))
    our_blocks = {ex.block_key(b["name"]) for b in target.get("blocks", [])}
    report = {"layers": [p for p in layer_paths(ours["layers"]) if p not in before],
              "blocks": [b["name"] for b in ours["blocks"] if ex.block_key(b["name"]) not in our_blocks and b is not made],
              "kept": [], "styles": [], "block": made["name"]}
    our_styles = {ex.fold(s["name"]) for s in target["settings"].get("textStyles", []) + target["settings"].get("dimensionStyles", [])}
    report["styles"] = [s["name"] for s in ours["settings"].get("textStyles", []) + ours["settings"].get("dimensionStyles", [])
                        if ex.fold(s["name"]) not in our_styles]
    report["kept"] = []
    return ours, [insert], report


def without_ids(blocks):
    out = []
    for b in blocks:
        b = {k: v for k, v in b.items() if k != "id"}
        out.append(b)
    return out


def written(target, ours, objects, report, inp, warnings=(), op="execute"):
    first = max(e["id"] for e in target["entities"]) + 1
    made = {}
    for i, o in enumerate(objects):
        made[str(first + i)] = {**o, "id": first + i}
    step = {"op": op, "input": inp}
    if op == "plan":
        step["result"] = {"status": "completed", "warnings": list(warnings), "output": {
            "created": [{**o, "id": 0} for o in objects], "report": report, "revision": "$current"}}
        step["expect"] = {"revision": "same"}
        return step
    step["result"] = {"status": "completed", "warnings": list(warnings), "output": {
        "created": [U(first + i) for i in range(len(objects))], "report": report, "revision": "$current"}}
    expect = {"ids": [e["id"] for e in target["entities"]] + [first + i for i in range(len(objects))],
              "entities": made, "revision": "changed", "canUndo": True, "layers": ours["layers"]}
    if ours.get("blocks") != target.get("blocks"):
        expect["blocks"] = without_ids(ours["blocks"])
    styles = {}
    for key in ("textStyles", "dimensionStyles"):
        if ours["settings"].get(key) != target["settings"].get(key):
            styles[key] = ours["settings"].get(key)
    if styles:
        expect["settings"] = styles
    step["expect"] = expect
    return step


def units(a, b):
    return {"code": "units_differ", "path": "clip.unit",
            "message": f"Kaynak çizimin birimi {a}, bu çizimin {b}; nesneler gerçek boylarıyla yapıştırıldı."}


def free_name(name, taken, key):
    n = 2
    while key(f"{name} ({n})") in taken:
        n += 1
    return f"{name} ({n})"


def swap(v, old, new):
    if isinstance(v, str):
        return new if v == old else v
    if isinstance(v, list):
        return [swap(x, old, new) for x in v]
    if isinstance(v, dict):
        return {k: swap(x, old, new) for k, x in v.items()}
    return v


def renamed_clip(target, clip):
    """ADR 0351 §4: the copy with its names this drawing has renamed, and what was renamed."""
    clip = copy.deepcopy(clip)
    out = []
    ours = {ex.block_key(b["name"]) for b in target.get("blocks", [])}
    taken = ours | {ex.block_key(b["name"]) for b in clip.get("blocks", [])}
    ids = []
    for b in clip.get("blocks", []):
        if ex.block_key(b["name"]) not in ours:
            continue
        to = free_name(b["name"], taken, ex.block_key)
        taken.add(ex.block_key(to))
        out.append({"kind": "block", "from": b["name"], "to": to})
        ids.append((b["id"], f"$blockOf:{to}"))
        b["name"] = to
    for old, new in ids:
        clip["objects"] = swap(clip["objects"], old, new)
        clip["blocks"] = swap(clip["blocks"], old, new)
    for kind, key in (("textStyle", "textStyles"), ("dimensionStyle", "dimensionStyles")):
        mine = {ex.fold(s_["name"]) for s_ in target["settings"].get(key, [])}
        taken = mine | {ex.fold(s_["name"]) for s_ in clip.get(key, [])}
        for s_ in clip.get(key, []):
            if ex.fold(s_["name"]) not in mine:
                continue
            to = free_name(s_["name"], taken, ex.fold)
            taken.add(ex.fold(to))
            out.append({"kind": kind, "from": s_["name"], "to": to})
            s_["name"] = to
    return clip, out


def paste(name, target, clip, at=None, as_block=None, warnings=(), op="execute", undo=False, rename=False):
    inp = {"clip": clip}
    if rename:
        inp["conflicts"] = "rename"
        clip, renames = renamed_clip(target, clip)
    if at is not None:
        inp["at"] = at
    if as_block is not None:
        inp["asBlock"] = as_block
    if as_block is not None:
        ours, objects, report = paste_as_block(target, clip, as_block.strip(), at)
    else:
        ours, objects, report = paste_of(target, clip, at)
    if rename:
        report["renamed"] = renames
        said = {"code": "names_renamed", "path": "conflicts", "message": "Bu çizimde aynı adlı olduklarından yeni adla geldi: "
                + ", ".join(f"{r['from']} → {r['to']}" for r in renames) + "."}
        units_at = next((k for k, w in enumerate(warnings) if w["code"] == "units_differ"), len(warnings))
        warnings = [*warnings[:units_at], said, *warnings[units_at:]]
    steps = [written(target, ours, objects, report, inp, warnings, op)]
    if undo:
        steps.append({"op": "undo", "returns": LABEL,
                      "expect": {"ids": [e["id"] for e in target["entities"]], "layers": target["layers"]}})
    case = {"name": name, "steps": steps}
    if target is not A:
        case["setup"] = target
    return case


def paste_refused(name, target, inp, code, message, path, extra=None):
    case = {"name": name, "steps": [refused(inp, code, message, path, extra)]}
    if target is not A:
        case["setup"] = target
    return case


def source_ids(clip):
    """A copy as it reaches another drawing: its ids the source drawing's own (here fixed UUIDs)."""
    clip = copy.deepcopy(clip)
    clip["uids"] = [f"0192f5a3-0000-7000-8000-{int(u.split(':')[1]):012d}" for u in clip["uids"]]
    return clip


def paste_cases():
    mm = units("m", "mm")
    line = source_ids(copy_of(A, [1]))
    area = source_ids(copy_of(A, [2]))
    text = source_ids(copy_of(A, [3], at=P(0, 0)))
    pit = source_ids(copy_of(A, [4], at=P(30, 0)))
    dim = source_ids(copy_of(A, [5], at=P(0, -6)))
    three = source_ids(copy_of(A, [1, 2, 3]))
    bad_layer = copy.deepcopy(line)
    bad_layer["objects"][0]["layerId"] = "yok"
    bad_block = copy.deepcopy(pit)
    bad_block["blocks"] = []
    house = {**copy.deepcopy(line), "layers": [node("yapi-a", "Yapı")]}
    house["objects"][0]["layerId"] = "yapi-a"
    locked = source_ids(copy_of(A, [6]))
    hidden = source_ids(copy_of(A, [7]))
    return [
        paste("Aynı çizime başka yere: katmanı var, bir şey eklenmez; adım “Yapıştır”, geri alınır", A, line,
              at=P(50, 0), undo=True),
        paste("Başka çizime (milimetre): çizgi “Çizim”e, boyları korunur, birim söylenir", B, line, at=P(100, 0),
              warnings=[mm]),
        paste("Olmayan katman yolu açılır: “KADASTRO”nun altında “Parsel” (adlar büyük küçük harf gözetmeden)", B, area,
              warnings=[mm]),
        paste("Aynı adlı yazı stili bu çiziminki: yazı onu kullanır", B, text, at=P(0, 50), warnings=[mm]),
        paste("Aynı adlı blok bu çizimin tanımıyla: “Rögar” korunur, iç bloğu eklenir", B, pit, at=P(0, 60),
              warnings=[mm]),
        paste("Olmayan ölçü stili eklenir", B, dim, at=P(0, 70), warnings=[mm]),
        paste("Üç katmanlı kopya: nesneler kopyanın sırasıyla", B, three, at=P(200, 0), warnings=[mm]),
        paste("Özgün koordinatlarda: taban noktası yerinde", B, line, warnings=[mm]),
        paste("Blok olarak: tek blok, tabanı nesnelerin sol alt köşesi, etkin katmanda", B, three, at=P(300, 0),
              as_block="Pano bloğu", warnings=[mm]),
        paste("Blok olarak, ad varsa boş bir ad alır", B, line, at=P(400, 0), as_block="rögar", warnings=[mm]),
        paste("Gizli katmana: yapıştırılır, söylenir", B, hidden, at=P(0, 80),
              warnings=[{"code": "layer_hidden", "path": "clip",
                         "message": "“Gizli” katmanı gizli; yapıştırılan nesneler görünmeyecek."}, mm]),
        paste("Plan yazmadan gösterir", B, three, at=P(200, 0), warnings=[mm], op="plan"),
        paste("Adları yenile: aynı adlı blok kopyanınkiyle “Rögar (2)” olarak gelir", B, pit, at=P(0, 60),
              warnings=[mm], rename=True),
        paste("Adları yenile: aynı adlı yazı stili “Başlık (2)” olarak gelir, yazı onu kullanır", B, text, at=P(0, 50),
              warnings=[mm], rename=True),
        paste("Adları yenile: aynı çizime, iç içe iki blok yeni adlarla, iç yerleştirme yenisini izler", A, pit,
              at=P(40, 0), rename=True, undo=True),
        paste_refused("Kilitli katmana: layer_locked", B, {"clip": locked, "at": P(0, 0)}, "layer_locked",
                      "“Kilitli” katmanı kilitli; yapıştırılamadı. Kilidi Katmanlar panelinden açın.", "clip"),
        paste_refused("Yolda bu çizimin grubu var: not_a_layer", B, {"clip": house},
                      "not_a_layer", "“Yapı” alınamadı: kaynakta böyle bir katman yok ya da bu çizimde aynı yolda katman olmayan bir düğüm var.",
                      "clip.layers"),
        paste_refused("Boş kopya: no_entities", B, {"clip": {**line, "objects": [], "uids": []}}, "no_entities",
                      "Pano boş: yapıştırılacak nesne yok. Önce nesneleri kopyalayın.", "clip.objects"),
        paste_refused("Kimlik sayısı nesnelerinkinden başka: invalid_clip", B, {"clip": {**line, "uids": []}},
                      "invalid_clip", "Pano içeriğinde 1 nesne var ama 0 kimlik; her nesnenin bir kimliği olmalı.",
                      "clip.uids"),
        paste_refused("Nesnenin katmanı kopyada yok: invalid_clip", B, {"clip": bad_layer}, "invalid_clip",
                      "Pano içeriğinin 1. nesnesinin katmanı (“yok”) içinde yok.", "clip.objects[0].layerId"),
        paste_refused("Yerleştirmenin bloğu kopyada yok: invalid_clip", B, {"clip": bad_block}, "invalid_clip",
                      "Pano içeriğinin 1. nesnesinin bloğu içinde yok.", "clip.objects[0].block"),
        paste_refused("Yapıştırma noktası sayı değil: not_finite", B, {"clip": line, "at": P(0, 0)}, "not_finite",
                      "Yapıştırma noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", "at.x", {"nonFinite": {"at.x": "NaN"}}),
        paste_refused("Blok adı boş: empty_name", B, {"clip": line, "asBlock": "  "}, "empty_name",
                      "Bloğun adı boş; bir ad verin.", "asBlock"),
        paste_refused("Beklenen sürüm sayı değil: invalid_revision", B, {"clip": line, "expectedRevision": "on iki"},
                      "invalid_revision", "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Çizim değiştiyse conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "command": "cad.entities.transform",
             "input": {"uids": [U(1)], "transform": {"kind": "move", "dx": 1, "dy": 0}}, "result": "$any"},
            {"op": "execute", "input": {"clip": line, "at": P(50, 0), "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                            "hâline göre yeniden hazırlayın."}},
             "expect": {"revision": "same"}}]},
    ]


def build(command):
    make, title = {
        "cad.entities.copy": (copy_cases, "Panoya kopyala: nesneler kullandıklarıyla"),
        "cad.entities.paste": (paste_cases, "Yapıştır: katmanlar, bloklar ve stiller adıyla"),
    }[command]
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 0309. Bağımsız başvuru: scripts/fixtures/clipboard_command_cases.py (alışverişin kuralları "
                    "exchange_cases.py'den).",
            "setup": A, "cases": make()}


def main():
    check = "--check" in sys.argv[1:]
    bad = 0
    for command in ("cad.entities.copy", "cad.entities.paste"):
        path = OUT / f"{command}.json"
        data = build(command)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return bad


if __name__ == "__main__":
    sys.exit(main())
