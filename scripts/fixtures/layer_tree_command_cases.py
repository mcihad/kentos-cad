"""Writes the shared cases of the layer tree's three commands (docs/adr/0248 §1–§3):

- fixtures/commands/v1/cad.layers.edit.json   (`cad.layers.edit` v1: add, style, move, remove, duplicate, merge, fields, purge)
- fixtures/commands/v1/cad.layers.set.json    (`cad.layers.set` v1: rename, visible, locked, active, isolate, showAll)
- fixtures/commands/v1/cad.layers.states.json (`cad.layers.states` v1: save, update, rename, remove, apply)

The expectations are built here from the contract's rules (crates/shared/contracts/src/cad_layer_tree.rs: the checks and
their order, the codes, paths and words, the steps' names) and from the ADRs' meaning of each change (docs/adr/0072 a
layer removed with its objects, 0076 a layer added, 0177 a copy, a merge, the purge and the states, 0199 the fields,
0020 the tree's own changes), with the tree, the objects and the states modelled below as plain JSON, without KentOS
code. The purge's rule is written again here from docs/adr/0177 §5 (and §1 of 0248: a layer drawn from a map service
is in use), the states' from §4. The desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them; the web will on its return (TODOS.md §0.2).

    python3 scripts/fixtures/layer_tree_command_cases.py           # writes the files
    python3 scripts/fixtures/layer_tree_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def _load(name):
    """Another independent reference of this folder, read as a module (its rules, not KentOS code)."""
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FIELDS = _load("layer_field_cases")

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
BLUE = {"color": "#0090FF", "lineType": "continuous", "lineWeight": 0.35}
DEFAULT = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
ADA = {"name": "Ada", "kind": "text"}
X, Y = 500000, 4400000

INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def node(id_, name, type_="layer", children=None, style=STYLE, visible=True, locked=False, expanded=True, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": visible, "locked": locked, "expanded": expanded,
            "style": dict(style), "children": children or [], **copy.deepcopy(extra)}


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


AGAC = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2101"
LAMBA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2102"
BLOCKS = [
    {"id": AGAC, "name": "Ağaç", "base": {"x": 0, "y": 0}, "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": {"x": 0, "y": 0}, "r": 1.5}]},
    {"id": LAMBA, "name": "Lamba", "base": {"x": 0, "y": 0}, "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": {"x": 0, "y": 0}, "r": 0.25}]},
]


def blocks_without_ids(ids):
    out = []
    for b in BLOCKS:
        if b["id"] in ids:
            d = copy.deepcopy(b)
            d.pop("id")
            out.append(d)
    return out


def edit_layers():
    return [
        node("kadastro", "Kadastro", "group", [node("parsel", "Parsel", style=BLUE, fields=[ADA]), node("bina", "Bina", locked=True)]),
        node("yol", "Yol"),
        node("bos", "Boş"),
        node("arsiv", "Arşiv", "group", [node("eski", "Eski")], expanded=False),
        node("altlik", "OpenStreetMap", service=OSM),
        node("koruma", "Koruma alanı", "group", [node("sit", "Sit alanı")], locked=True),
        node("gizli", "Gizli", visible=False),
    ]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "color": "#E5484D", "attrs": {"Ada": "101", "Nitelik": "Arsa"}, "pts": rect(X, Y, 40, 30)},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "attrs": {"Ada": "102"}, "pts": rect(X + 40, Y, 20, 20)},
    {"kind": "polygon", "id": 3, "layerId": "bina", "attrs": {}, "pts": rect(X + 5, Y + 5, 10, 10)},
    {"kind": "line", "id": 4, "layerId": "yol", "attrs": {}, "a": {"x": X - 10, "y": Y + 40}, "b": {"x": X + 120, "y": Y + 40}},
    {"kind": "insert", "id": 5, "layerId": "yol", "attrs": {"NO": "A-1"}, "block": AGAC, "p": {"x": X + 50, "y": Y + 45}, "scale": 1, "rotation": 0},
    {"kind": "point", "id": 6, "layerId": "gizli", "attrs": {}, "p": {"x": X + 10, "y": Y + 60}},
]
IDS = [e["id"] for e in ENTITIES]


def setup(layers, entities=ENTITIES, active="parsel", **settings):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Katman ağacı",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "cad", "drawingFont": "barlow", **copy.deepcopy(settings)},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(layers),
        "activeLayer": active,
        "entities": copy.deepcopy(entities),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── The tree as plain JSON ───────────────────────────────────────────────────

def walk(nodes, parent=None):
    """Every node in the tree's order (a group before what it holds), with its group."""
    for n in nodes:
        yield n, parent
        yield from walk(n["children"], n)


def get(nodes, id_):
    return next((n for n, _ in walk(nodes) if n["id"] == id_), None)


def parent_of(nodes, id_):
    return next((p for n, p in walk(nodes) if n["id"] == id_), None)


def list_of(nodes, group):
    return nodes if group is None else get(nodes, group)["children"]


def place_of(nodes, id_):
    p = parent_of(nodes, id_)
    lst = list_of(nodes, p["id"] if p else None)
    return (p["id"] if p else None, [n["id"] for n in lst].index(id_))


def leaves(n):
    if n["type"] == "layer":
        return [n["id"]]
    return [i for c in n["children"] for i in leaves(c)]


def locked(nodes, id_):
    """Locked by itself or a group above it."""
    n, chain = get(nodes, id_), []
    while n is not None:
        chain.append(n)
        n = parent_of(nodes, n["id"])
    return any(x["locked"] for x in chain)


def changed(nodes, id_, **fields):
    out = copy.deepcopy(nodes)
    get(out, id_).update(copy.deepcopy(fields))
    return out


def removed(nodes, id_):
    out = copy.deepcopy(nodes)
    p = parent_of(out, id_)
    lst = list_of(out, p["id"] if p else None)
    lst[:] = [n for n in lst if n["id"] != id_]
    return out


def inserted(nodes, n, group, index=None):
    out = copy.deepcopy(nodes)
    lst = list_of(out, group)
    lst.insert(len(lst) if index is None else min(index, len(lst)), copy.deepcopy(n))
    return out


def container(nodes, parent):
    """The group a node added beside or into `parent` goes into (docs/adr/0076): `parent` when a group, its group
    when a layer, the top when none."""
    if parent is None:
        return None
    n = get(nodes, parent)
    if n["type"] == "group":
        return n["id"]
    p = parent_of(nodes, parent)
    return p["id"] if p else None


def names(nodes):
    return {n["name"] for n, _ in walk(nodes)}


def unique(nodes, base):
    """`base`, else `base 2`, `base 3` … (one of its kind among every node's name)."""
    if base not in names(nodes):
        return base
    i = 2
    while f"{base} {i}" in names(nodes):
        i += 1
    return f"{base} {i}"


def moved(nodes, id_, group, index=None):
    """A node with everything under it out of its place, into `group` at `index` among what is left there."""
    n = copy.deepcopy(get(nodes, id_))
    return inserted(removed(nodes, id_), n, group, index)


# ── The objects ──────────────────────────────────────────────────────────────

def on(entities, layer):
    return [e for e in entities if e["layerId"] == layer]


def by_id(entities):
    return {str(e["id"]): e for e in entities}


# ── Steps ────────────────────────────────────────────────────────────────────

def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


VALID = {"status": "completed", "output": None, "warnings": []}


def unchanged(layers, ids=IDS):
    return {"ids": ids, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "layers": layers}


def refused(name, input_, code, message, path, layers, ids=IDS):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), unchanged(layers, ids))]}


def conflict_case(name, first, second, after):
    """A write, then one prepared against the revision before it: nothing written, `conflict`."""
    return {
        "name": name,
        "steps": [
            step("captureRevision", **{"as": "once"}),
            first,
            step("execute", {**second, "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", **after}),
        ],
    }


def not_found(id_):
    return f"“{id_}” kimlikli katman ya da grup çizimde yok. Var olan bir düğümün kimliğini verin."


# ── cad.layers.edit ──────────────────────────────────────────────────────────

def edited(layer=None, changed_=True, objects=0, removed_=(), blocks=(), kept=0):
    """`cad.layers.edit`'s answer; `layer` is `$layer` for a node the command adds."""
    out = {"changed": changed_, "objects": objects, "removed": list(removed_), "blocks": list(blocks), "kept": kept, "revision": "$current"}
    if layer is not None:
        out["layer"] = layer
    return completed(out)


def purge_rule(nodes, entities, blocks, active, asked_layers, asked_groups, asked_blocks):
    """docs/adr/0177 §5 over the asked ones alone (0248 §1: a layer drawn from a map service stays): what goes, in the
    order it goes (layers in the tree's order, groups deepest first, definitions in rounds), and how many asked stay."""
    can = set()
    for n, _ in walk(nodes):
        if locked(nodes, n["id"]):
            continue
        if n["type"] == "layer" and n["id"] != active and "service" not in n:
            can.add(("layers", n["id"]))
        if n["type"] == "group":
            can.add(("groups", n["id"]))
    for b in blocks:
        can.add(("blocks", b["id"]))
    asked = {("layers", i) for i in asked_layers} | {("groups", i) for i in asked_groups} | {("blocks", i) for i in asked_blocks}
    going = asked & can
    while True:
        used = set()
        for e in entities:
            used.add(("layers", e["layerId"]))
            if e["kind"] == "insert":
                used.add(("blocks", e["block"]))
        for b in blocks:
            if ("blocks", b["id"]) not in going:
                for e in b["entities"]:
                    used.add(("layers", e["layerId"]))
                    if e["kind"] == "insert":
                        used.add(("blocks", e["block"]))
        for n, p in walk(nodes):
            kind = "layers" if n["type"] == "layer" else "groups"
            if (kind, n["id"]) not in going and p is not None:
                used.add(("groups", p["id"]))
        before = len(going)
        going -= used
        if len(going) == before:
            break
    gone_layers = [n["id"] for n, _ in walk(nodes) if n["type"] == "layer" and ("layers", n["id"]) in going]

    def deepest(ns, out):
        for n in ns:
            deepest(n["children"], out)
            if n["type"] == "group" and ("groups", n["id"]) in going:
                out.append(n["id"])
        return out

    gone_groups = deepest(nodes, [])
    left = [b for b in blocks if ("blocks", b["id"]) in going]
    gone_blocks = []
    while left:
        inside = {e["block"] for b in left for e in b["entities"] if e["kind"] == "insert"}
        now = [b["id"] for b in left if b["id"] not in inside]
        if not now:
            break
        gone_blocks += now
        left = [b for b in left if b["id"] not in now]
    return gone_layers, gone_groups, gone_blocks, len(asked) - len(going)


def found_all(nodes, entities, blocks, active):
    """What the purge takes when nothing is named: every unlocked layer and group and every definition."""
    layers = [n["id"] for n, _ in walk(nodes) if n["type"] == "layer" and not locked(nodes, n["id"])]
    groups = [n["id"] for n, _ in walk(nodes) if n["type"] == "group" and not locked(nodes, n["id"])]
    gl, gg, gb, _ = purge_rule(nodes, entities, blocks, active, layers, groups, [b["id"] for b in BLOCKS])
    return gl, gg, gb


def edit_cases():
    t = edit_layers()
    out = []
    E = by_id(ENTITIES)

    # add
    new = node("$layer", "Yeni katman", style=DEFAULT)
    after = inserted(t, new, None)
    out.append({"name": "Ekle: adı verilmeyen katman “Yeni katman”, en üstte en sona, varsayılan stille; adımı “Katman ekle”",
                "steps": [step("execute", {"operation": "add"}, edited("$layer"),
                               {"ids": IDS, "layers": after, "canUndo": True, "dirty": True, "revision": "changed", "activeLayer": "parsel"}),
                          step("undo", returns="Katman ekle", expect={"layers": t, "canRedo": True}),
                          step("redo", returns="Katman ekle", expect={"layers": after})]})
    taken = inserted(t, node("yeni", "Yeni katman"), None)
    out.append({"name": "Ekle: “Yeni katman” varsa “Yeni katman 2”",
                "setup": setup(taken),
                "steps": [step("execute", {"operation": "add"}, edited("$layer"),
                               {"layers": inserted(taken, node("$layer", unique(taken, "Yeni katman"), style=DEFAULT), None)})]})
    group = node("$layer", "Mimari", "group", style=DEFAULT)
    out.append({"name": "Ekle: grup; adımı “Grup ekle”; grup etkin katman olmaz",
                "steps": [step("execute", {"operation": "add", "kind": "group", "name": "  Mimari ", "activate": True}, edited("$layer"),
                               {"layers": inserted(t, group, None), "activeLayer": "parsel"}),
                          step("undo", returns="Grup ekle", expect={"layers": t})]})
    duvar = node("$layer", "Duvar", style={"color": "#FF0000", "lineType": "dashed", "lineWeight": 0.5})
    into = container(t, "bina")
    out.append({"name": "Ekle: bir katmanın yanına (grubuna) ilk sıraya, stiliyle, etkin; geri alma etkin katmanı geri verir",
                "steps": [step("execute", {"operation": "add", "name": "Duvar", "parent": "bina", "index": 0, "color": "#FF0000", "lineType": "dashed",
                                           "lineWeight": 0.5, "activate": True}, edited("$layer"),
                               {"layers": inserted(t, duvar, into, 0), "activeLayer": "$layer"}),
                          step("undo", returns="Katman ekle", expect={"layers": t, "activeLayer": "parsel"})]})
    opened = changed(t, "arsiv", expanded=True)
    hidden_new = node("$layer", "Taslak", style=DEFAULT, visible=False, locked=True)
    out.append({"name": "Ekle: kapalı gruba giren düğüm grubu açar; geri almada grup açık kalır (görünüm); gizli ve kilitli verilebilir",
                "steps": [step("execute", {"operation": "add", "name": "Taslak", "parent": "eski", "visible": False, "locked": True}, edited("$layer"),
                               {"layers": inserted(opened, hidden_new, "arsiv")}),
                          step("undo", returns="Katman ekle", expect={"layers": opened})]})
    out.append({"name": "Plan: eklenecek katmanın kimliği; yazmaz",
                "steps": [step("plan", {"operation": "add", "name": "Duvar"}, edited("$layer"), unchanged(t))]})
    out.append(refused("Ekle: boş ad", {"operation": "add", "name": "  "}, "empty_name", "Katmanın adı boş olamaz; bir ad yazın ya da adı vermeyin.", "name", t))
    out.append(refused("Ekle: çizimde olmayan üst düğüm", {"operation": "add", "parent": "yok"}, "layer_not_found", not_found("yok"), "parent", t))
    out.append(refused("Ekle: renk değil", {"operation": "add", "color": "kırmızı"}, "invalid_color",
                       "“kırmızı” bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın.", "color", t))
    out.append(refused("Ekle: kalınlık 100 mm'den büyük", {"operation": "add", "lineWeight": 150}, "invalid_line_weight",
                       "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight", t))

    # style
    for given, label, why in (
        ({"color": "#00AA00"}, "Katman rengi", "yalnız renk"),
        ({"lineType": "dashdot"}, "Çizgi tipi", "yalnız çizgi tipi"),
        ({"lineWeight": 0.7}, "Çizgi kalınlığı", "yalnız kalınlık"),
        ({"color": "ink", "lineWeight": 0.7}, "Katman stili", "birden çok"),
    ):
        style = {**BLUE, **given}
        after = changed(t, "parsel", style=style)
        out.append({"name": f"Stil: {why}; adımı “{label}”",
                    "steps": [step("execute", {"operation": "style", "layer": "parsel", **given}, edited("parsel"),
                                   {"layers": after, "canUndo": True, "ids": IDS}),
                              step("undo", returns=label, expect={"layers": t})]})
    out.append({"name": "Stil: kilitli katman ve grup da alır (görünüştür)",
                "steps": [step("execute", {"operation": "style", "layer": "bina", "color": "fg-dim"}, edited("bina"),
                               {"layers": changed(t, "bina", style={**STYLE, "color": "fg-dim"})}),
                          step("execute", {"operation": "style", "layer": "koruma", "lineType": "dotted"}, edited("koruma"),
                               {"layers": changed(changed(t, "bina", style={**STYLE, "color": "fg-dim"}), "koruma", style={**STYLE, "lineType": "dotted"})})]})
    out.append({"name": "Stil: aynı değerler bir şey yazmaz",
                "steps": [step("execute", {"operation": "style", "layer": "parsel", "color": "#0090FF"}, edited("parsel", False), unchanged(t))]})
    out.append(refused("Stil: katmansız", {"operation": "style", "color": "ink"}, "no_layer", "İşlemin katmanını ya da grubunu kimliğiyle verin (layer).", "layer", t))
    out.append({"name": "Stil: değişecek bir şey yok",
                "steps": [step("execute", {"operation": "style", "layer": "parsel"},
                               failed("nothing_to_set", "Değişecek bir şey yok: color, lineType ya da lineWeight verin."), unchanged(t))]})

    # move
    after = moved(t, "yol", "kadastro", 0)
    out.append({"name": "Taşı: katman bir grubun başına; adımı “Katmanı taşı”; nesneler katmanında kalır",
                "steps": [step("execute", {"operation": "move", "layer": "yol", "parent": "kadastro", "index": 0}, edited("yol"),
                               {"layers": after, "ids": IDS, "entities": {"4": E["4"]}, "canUndo": True}),
                          step("undo", returns="Katmanı taşı", expect={"layers": t}),
                          step("redo", returns="Katmanı taşı", expect={"layers": after})]})
    after = moved(t, "kadastro", None)
    out.append({"name": "Taşı: grup altındakilerle en alta (parent ve index yok); etkin katman değişmez",
                "steps": [step("execute", {"operation": "move", "layer": "kadastro"}, edited("kadastro"), {"layers": after, "activeLayer": "parsel"}),
                          step("undo", returns="Katmanı taşı", expect={"layers": t})]})
    after = moved(t, "bos", None, 0)
    out.append({"name": "Taşı: aynı grupta yukarı",
                "steps": [step("execute", {"operation": "move", "layer": "bos", "index": 0}, edited("bos"), {"layers": after}),
                          step("undo", returns="Katmanı taşı", expect={"layers": t})]})
    _, here = place_of(t, "yol")
    out.append({"name": "Taşı: aynı yere bir şey yazmaz",
                "steps": [step("execute", {"operation": "move", "layer": "yol", "index": here}, edited("yol", False), unchanged(t))]})
    out.append(refused("Taşı: üst düğüm katman", {"operation": "move", "layer": "yol", "parent": "parsel"}, "not_a_group",
                       "“Parsel” bir katman; düğüm yalnız bir grubun içine taşınır. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).", "parent", t))
    out.append(refused("Taşı: kendi içine", {"operation": "move", "layer": "kadastro", "parent": "kadastro"}, "move_into_itself",
                       "“Kadastro” kendi içine ya da altındaki bir gruba taşınamaz.", "parent", t))
    out.append(refused("Taşı: kilitli katman", {"operation": "move", "layer": "bina", "parent": "arsiv"}, "layer_locked",
                       "“Bina” katmanı kilitli; taşınamaz. Kilidini Katmanlar panelinden açın.", "layer", t))
    out.append(refused("Taşı: kilitli gruba", {"operation": "move", "layer": "yol", "parent": "koruma"}, "layer_locked",
                       "“Koruma alanı” grubu kilitli; içine düğüm taşınamaz. Kilidini Katmanlar panelinden açın.", "parent", t))
    out.append(refused("Taşı: çizimde olmayan düğüm", {"operation": "move", "layer": "yok"}, "layer_not_found", not_found("yok"), "layer", t))

    # remove
    gone = [e for e in ENTITIES if e["layerId"] != "yol"]
    out.append({"name": "Sil: katman üzerindeki nesnelerle; adımı “Katman sil”; geri alma ikisini birden geri getirir",
                "steps": [step("execute", {"operation": "remove", "layer": "yol"}, edited("yol", objects=2),
                               {"ids": [e["id"] for e in gone], "layers": removed(t, "yol"), "canUndo": True}),
                          step("undo", returns="Katman sil", expect={"ids": IDS, "layers": t, "entities": {"4": E["4"], "5": E["5"]}})]})
    out.append({"name": "Sil: boş grup altındakilerle",
                "steps": [step("execute", {"operation": "remove", "layer": "arsiv"}, edited("arsiv"), {"ids": IDS, "layers": removed(t, "arsiv")})]})
    out.append(refused("Sil: etkin katman", {"operation": "remove", "layer": "parsel"}, "layer_refused",
                       "“Parsel” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.", "layer", t))
    out.append(refused("Sil: etkin katmanı içeren grup", {"operation": "remove", "layer": "kadastro"}, "layer_refused",
                       "“Kadastro” grubu etkin katmanı (“Parsel”) içeriyor; silinemez. Önce grubun dışındaki bir katmanı etkinleştirin.", "layer", t))
    out.append(refused("Sil: kilitli grup", {"operation": "remove", "layer": "koruma"}, "layer_refused",
                       "“Koruma alanı” grubu kilitli; silinemez. Kilidi Katmanlar panelinden açın.", "layer", t))

    # duplicate
    source = get(t, "parsel")
    copy_node = node("$layer", unique(t, "Parsel kopyası"), style=source["style"], fields=source["fields"])
    after = inserted(t, copy_node, container(t, "parsel"))
    copies = [dict(e, id=7 + i, layerId="$layer:1") for i, e in enumerate(on(ENTITIES, "parsel"))]
    out.append({"name": "Kopyasını oluştur: aynı gruba “Parsel kopyası”, stili, alanları ve nesnelerin kopyalarıyla; adımı “Katmanı kopyala”",
                "steps": [step("execute", {"operation": "duplicate", "layer": "parsel"}, edited("$layer", objects=2),
                               {"ids": IDS + [7, 8], "entities": by_id(copies), "uids": {"7": "new", "8": "new"}, "layers": after, "canUndo": True}),
                          step("undo", returns="Katmanı kopyala", expect={"ids": IDS, "layers": t})]})
    gizli = get(t, "gizli")
    after = inserted(t, node("$layer", "Gizli 2024", style=gizli["style"], visible=False), None)
    out.append({"name": "Kopyasını oluştur: verilen adla; görünürlüğü kaynağınki",
                "steps": [step("execute", {"operation": "duplicate", "layer": "gizli", "name": "Gizli 2024"},
                               {**edited("$layer", objects=1), "warnings": [{"code": "layer_hidden", "message": "“Gizli 2024” katmanı gizli; çizilen nesne görünmeyecek.",
                                                                              "path": "layerId"}]},
                               {"ids": IDS + [7], "entities": {"7": dict(E["6"], id=7, layerId="$layer:1")}, "layers": after})]})
    out.append(refused("Kopyasını oluştur: kilitli katman", {"operation": "duplicate", "layer": "bina"}, "layer_locked",
                       "“Bina” katmanı kilitli; kopyası oluşturulmaz. Kilidini Katmanlar panelinden açın.", "layer", t))
    out.append(refused("Kopyasını oluştur: grup", {"operation": "duplicate", "layer": "kadastro"}, "not_a_layer",
                       "“Kadastro” bir katman grubu; bu işlem yalnız katmanın olur. Grubun bir katmanını verin.", "layer", t))
    out.append(refused("Kopyasını oluştur: servis katmanı", {"operation": "duplicate", "layer": "altlik"}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; bu işlem nesneleri olan katmanın olur.", "layer", t))

    # merge
    merged_entities = {str(e["id"]): dict(e, layerId="yol") for e in on(ENTITIES, "parsel")}
    after = removed(t, "parsel")
    out.append({"name": "Birleştir: etkin katman kaynaksa hedef etkin olur (adımın dışında); nesneler hedefe, kaynak silinir; adımı “Katmanları birleştir”",
                "steps": [step("execute", {"operation": "merge", "layer": "yol", "layers": ["parsel", "yol", "parsel"]},
                               edited("yol", objects=2, removed_=["parsel"]),
                               {"ids": IDS, "entities": merged_entities, "layers": after, "activeLayer": "yol", "canUndo": True}),
                          step("undo", returns="Katmanları birleştir", expect={"ids": IDS, "layers": t, "entities": {"1": E["1"], "2": E["2"]}, "activeLayer": "yol"})]})
    out.append({"name": "Birleştir: boş katmanlar da; nesnesiz",
                "steps": [step("execute", {"operation": "merge", "layer": "yol", "layers": ["bos", "eski"]}, edited("yol", removed_=["bos", "eski"]),
                               {"ids": IDS, "layers": removed(removed(t, "bos"), "eski")})]})
    out.append(refused("Birleştir: kaynak yok (yalnız hedef)", {"operation": "merge", "layer": "yol", "layers": ["yol"]}, "no_layers",
                       "Hedefe birleşecek katmanları kimlikleriyle verin (layers); hedefin kendisi sayılmaz.", "layers", t))
    out.append(refused("Birleştir: hedef kilitli", {"operation": "merge", "layer": "bina", "layers": ["bos"]}, "layer_locked",
                       "“Bina” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.", "layer", t))
    out.append(refused("Birleştir: kaynak grup", {"operation": "merge", "layer": "yol", "layers": ["bos", "arsiv"]}, "not_a_layer",
                       "“Arşiv” bir katman grubu; bu işlem yalnız katmanın olur. Grubun bir katmanını verin.", "layers[1]", t))
    out.append(refused("Birleştir: kaynak kilitli (grubundan)", {"operation": "merge", "layer": "yol", "layers": ["sit"]}, "layer_locked",
                       "“Sit alanı” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.", "layers[0]", t))

    # fields
    nitelik = {"name": "Nitelik", "kind": "text"}
    after = changed(t, "parsel", fields=[ADA, nitelik])
    out.append({"name": "Alanlar: katmanın alanları; adımı “Alanlar”",
                "steps": [step("execute", {"operation": "fields", "layer": "parsel", "fields": [ADA, nitelik]}, edited("parsel"), {"layers": after, "canUndo": True}),
                          step("undo", returns="Alanlar", expect={"layers": t})]})
    ada_no = {"name": "Ada no", "kind": "text"}
    renamed = {str(e["id"]): dict(e, attrs={("Ada no" if k == "Ada" else k): v for k, v in e["attrs"].items()}) for e in on(ENTITIES, "parsel")}
    out.append({"name": "Alanlar: yeniden adlandırma nesnelerin anahtarlarını da değiştirir, değerler kalır",
                "steps": [step("execute", {"operation": "fields", "layer": "parsel", "fields": [ada_no], "renames": [{"from": "Ada", "to": "Ada no"}]},
                               edited("parsel"), {"layers": changed(t, "parsel", fields=[ada_no]), "entities": renamed}),
                          step("undo", returns="Alanlar", expect={"layers": t, "entities": {"1": E["1"], "2": E["2"]}})]})
    no_fields = copy.deepcopy(t)
    get(no_fields, "parsel").pop("fields")
    out.append({"name": "Alanlar: fields verilmezse alanlar kalkar; değerler öznitelik olarak kalır",
                "steps": [step("execute", {"operation": "fields", "layer": "parsel"}, edited("parsel"), {"layers": no_fields, "ids": IDS, "entities": {"1": E["1"]}})]})
    out.append({"name": "Alanlar: aynı alanlar bir şey yazmaz",
                "steps": [step("execute", {"operation": "fields", "layer": "parsel", "fields": [ADA]}, edited("parsel", False), unchanged(t))]})
    twice = [ADA, {"name": "ada", "kind": "integer"}]
    out.append(refused("Alanlar: aynı adlı iki alan", {"operation": "fields", "layer": "parsel", "fields": twice}, "invalid_fields",
                       FIELDS.fields_problem(twice), "fields", t))
    out.append(refused("Alanlar: grup", {"operation": "fields", "layer": "kadastro", "fields": [ADA]}, "not_a_layer",
                       "“Kadastro” bir katman grubu; bu işlem yalnız katmanın olur. Grubun bir katmanını verin.", "layer", t))
    out.append(refused("Alanlar: yeniden adlandırma bir nesnede aynı adlı iki öznitelik yapar",
                       {"operation": "fields", "layer": "parsel", "fields": [nitelik], "renames": [{"from": "Ada", "to": "Nitelik"}]}, "layer_refused",
                       "Yeniden adlandırma bir nesnede “Nitelik” adlı iki öznitelik yapıyor; alana başka bir ad verin.", "renames", t))

    # purge
    gl, gg, gb = found_all(t, ENTITIES, BLOCKS, "parsel")
    after = t
    for i in gl + gg:
        if get(after, i) is not None:
            after = removed(after, i)
    out.append({"name": "Temizle: boş katmanlar, boş kalan grup ve kullanılmayan blok; kilitli ve servis katmanı kalır; adımı “Kullanılmayanları temizle”",
                "steps": [step("execute", {"operation": "purge"}, edited(None, removed_=gl + gg, blocks=gb),
                               {"ids": IDS, "layers": after, "blocks": blocks_without_ids({b["id"] for b in BLOCKS} - set(gb)), "canUndo": True}),
                          step("undo", returns="Kullanılmayanları temizle", expect={"layers": t, "blocks": blocks_without_ids({AGAC, LAMBA})})]})
    gl, gg, gb, kept = purge_rule(t, ENTITIES, BLOCKS, "parsel", ["sit", "bos"], [], [])
    out.append({"name": "Temizle: yalnız adı verilenler; kilitli katman kalır ve sayılır; blocks boş: blok silinmez",
                "steps": [step("execute", {"operation": "purge", "layers": ["sit", "bos"], "blocks": []}, edited(None, removed_=gl + gg, blocks=gb, kept=kept),
                               {"layers": removed(t, "bos"), "blocks": blocks_without_ids({AGAC, LAMBA})})]})
    gl, gg, gb, kept = purge_rule(t, ENTITIES, BLOCKS, "parsel", [], ["arsiv"], [AGAC])
    out.append({"name": "Temizle: içindeki katman istenmeyen grup ve kullanılan blok kalır; bir şey yazılmaz",
                "steps": [step("execute", {"operation": "purge", "layers": ["arsiv"], "blocks": [AGAC]}, edited(None, False, removed_=gl + gg, blocks=gb, kept=kept),
                               unchanged(t))]})
    out.append(refused("Temizle: çizimde olmayan blok", {"operation": "purge", "blocks": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff"]}, "unknown_block",
                       "“0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff” kimlikli blok tanımı çizimde yok.", "blocks[0]", t))

    # validate, revision
    out.append({"name": "Doğrula: yazmaz", "steps": [step("validate", {"operation": "remove", "layer": "yol"}, VALID, unchanged(t))]})
    gl, gg, gb = found_all(t, ENTITIES, BLOCKS, "parsel")
    out.append({"name": "Plan: temizlenecekler; yazmaz", "steps": [step("plan", {"operation": "purge"}, edited(None, removed_=gl + gg, blocks=gb), unchanged(t))]})
    out.append(refused("geçersiz beklenen sürüm", {"operation": "remove", "layer": "yol", "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION,
                       "expectedRevision", t))
    out.append(refused("girdinin kuralları sürümden önce", {"operation": "merge", "layer": "yol", "expectedRevision": "on iki"}, "no_layers",
                       "Hedefe birleşecek katmanları kimlikleriyle verin (layers); hedefin kendisi sayılmaz.", "layers", t))
    out.append(conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                             step("execute", {"operation": "remove", "layer": "bos"}, edited("bos"), {"revision": "changed"}),
                             {"operation": "remove", "layer": "yol"}, {"layers": removed(t, "bos"), "ids": IDS}))
    return out


# ── cad.layers.set ───────────────────────────────────────────────────────────

def set_result(changed_=True, active="parsel"):
    return completed({"changed": changed_, "active": active, "revision": "$current"})


def edit_flags(layers):
    """A change of the tree's own: dirty, a new revision, no undo step."""
    return {"layers": layers, "ids": IDS, "dirty": True, "revision": "changed", "canUndo": False}


def isolate(nodes, ids):
    """These nodes, the groups above them and everything under them shown; the rest hidden."""
    out = copy.deepcopy(nodes)
    keep = set()
    for i in ids:
        n = get(out, i)
        stack = [n]
        while stack:
            x = stack.pop()
            keep.add(x["id"])
            stack.extend(x["children"])
        p = parent_of(out, i)
        while p is not None:
            keep.add(p["id"])
            p = parent_of(out, p["id"])
    for n, _ in walk(out):
        n["visible"] = n["id"] in keep
    return out


def set_cases():
    t = edit_layers()
    out = []
    out.append({"name": "Yeniden adlandır: ad kırpılır; düzenlemedir, geri alma adımı değildir",
                "steps": [step("execute", {"operation": "rename", "layer": "parsel", "name": "  Parsel 2 "}, set_result(), edit_flags(changed(t, "parsel", name="Parsel 2"))),
                          step("undo", returns=None, expect={"layers": changed(t, "parsel", name="Parsel 2")})]})
    out.append({"name": "Yeniden adlandır: aynı ad bir şey değiştirmez",
                "steps": [step("execute", {"operation": "rename", "layer": "parsel", "name": "Parsel"}, set_result(False), unchanged(t))]})
    out.append({"name": "Gizle: grup; altındakiler grubuyla gizlenir",
                "steps": [step("execute", {"operation": "visible", "layer": "kadastro", "value": False}, set_result(), edit_flags(changed(t, "kadastro", visible=False)))]})
    out.append({"name": "Göster: gizli katman",
                "steps": [step("execute", {"operation": "visible", "layer": "gizli", "value": True}, set_result(), edit_flags(changed(t, "gizli", visible=True)))]})
    out.append({"name": "Göster: zaten görünen bir şey değiştirmez",
                "steps": [step("execute", {"operation": "visible", "layer": "yol", "value": True}, set_result(False), unchanged(t))]})
    out.append({"name": "Kilitle: katman",
                "steps": [step("execute", {"operation": "locked", "layer": "yol", "value": True}, set_result(), edit_flags(changed(t, "yol", locked=True)))]})
    out.append({"name": "Kilidi aç: kilitli katman ve grup",
                "steps": [step("execute", {"operation": "locked", "layer": "bina", "value": False}, set_result(), edit_flags(changed(t, "bina", locked=False))),
                          step("execute", {"operation": "locked", "layer": "koruma", "value": False}, set_result(),
                               edit_flags(changed(changed(t, "bina", locked=False), "koruma", locked=False)))]})
    out.append({"name": "Kilitle: zaten kilitli bir şey değiştirmez",
                "steps": [step("execute", {"operation": "locked", "layer": "bina", "value": True}, set_result(False), unchanged(t))]})
    # Yazdır (docs/adr/0260 §4): only false is kept; true takes the field away.
    out.append({"name": "Yazdırma: katman yazdırılmaz; düzenlemedir, geri alma adımı değildir (ADR 0260 §4)",
                "steps": [step("execute", {"operation": "plot", "layer": "yol", "value": False}, set_result(), edit_flags(changed(t, "yol", plot=False))),
                          step("undo", returns=None, expect={"layers": changed(t, "yol", plot=False)})]})
    out.append({"name": "Yazdırma: grup altındakileriyle yazdırılmaz; yeniden açılınca alan kalkar",
                "steps": [step("execute", {"operation": "plot", "layer": "kadastro", "value": False}, set_result(), edit_flags(changed(t, "kadastro", plot=False))),
                          step("execute", {"operation": "plot", "layer": "kadastro", "value": True}, set_result(), edit_flags(t))]})
    out.append({"name": "Yazdırma: zaten yazdırılan bir şey değiştirmez",
                "steps": [step("execute", {"operation": "plot", "layer": "yol", "value": True}, set_result(False), unchanged(t))]})
    out.append(refused("Yazdırma: değersiz", {"operation": "plot", "layer": "parsel"}, "no_value",
                       "Açık mı kapalı mı olduğunu value ile verin (true ya da false).", "value", t))
    out.append(refused("Yazdırma: katmansız", {"operation": "plot", "value": False}, "no_layer", "Katmanı ya da grubu kimliğiyle verin (layer).", "layer", t))
    out.append({"name": "Etkin katman: dosyada saklanır ama düzenleme değildir (sürüm aynı)",
                "steps": [step("execute", {"operation": "active", "layer": "yol"}, set_result(True, "yol"),
                               {"layers": t, "activeLayer": "yol", "revision": "same", "dirty": False, "canUndo": False})]})
    out.append({"name": "Etkin katman: zaten etkin",
                "steps": [step("execute", {"operation": "active", "layer": "parsel"}, set_result(False), {**unchanged(t), "activeLayer": "parsel"})]})
    iso = isolate(t, ["parsel"])
    out.append({"name": "Yalıt: katman, üstündeki grup görünür; gerisi gizlenir",
                "steps": [step("execute", {"operation": "isolate", "layers": ["parsel"]}, set_result(), edit_flags(iso))]})
    iso2 = isolate(t, ["arsiv", "gizli"])
    out.append({"name": "Yalıt: grup altındakilerle, birden çok düğüm",
                "steps": [step("execute", {"operation": "isolate", "layers": ["arsiv", "gizli"]}, set_result(), edit_flags(iso2))]})
    all_shown = copy.deepcopy(t)
    for n, _ in walk(all_shown):
        n["visible"] = True
    out.append({"name": "Hepsini göster",
                "steps": [step("execute", {"operation": "showAll"}, set_result(), edit_flags(all_shown)),
                          step("execute", {"operation": "showAll"}, set_result(False), {"layers": all_shown, "revision": "same"})]})
    out.append({"name": "Plan: yazmaz", "steps": [step("plan", {"operation": "locked", "layer": "yol", "value": True}, set_result(), unchanged(t))]})
    out.append({"name": "Doğrula: yazmaz", "steps": [step("validate", {"operation": "showAll"}, VALID, unchanged(t))]})
    out.append(refused("katmansız", {"operation": "rename", "name": "A"}, "no_layer", "Katmanı ya da grubu kimliğiyle verin (layer).", "layer", t))
    out.append(refused("Yalıt: düğümsüz", {"operation": "isolate", "layers": []}, "no_layers",
                       "Görünecek katmanları ya da grupları kimlikleriyle verin (layers).", "layers", t))
    out.append(refused("Yeniden adlandır: boş ad", {"operation": "rename", "layer": "parsel", "name": " "}, "empty_name", "Katmanın adı boş olamaz; bir ad yazın.", "name", t))
    out.append(refused("Gizle: değersiz", {"operation": "visible", "layer": "parsel"}, "no_value",
                       "Açık mı kapalı mı olduğunu value ile verin (true ya da false).", "value", t))
    out.append(refused("çizimde olmayan düğüm", {"operation": "locked", "layer": "yok", "value": True}, "layer_not_found", not_found("yok"), "layer", t))
    out.append(refused("Yalıt: çizimde olmayan düğüm", {"operation": "isolate", "layers": ["parsel", "yok"]}, "layer_not_found", not_found("yok"), "layers[1]", t))
    out.append(refused("Etkin katman: grup", {"operation": "active", "layer": "kadastro"}, "not_a_layer",
                       "“Kadastro” bir katman grubu; etkin katman yalnız bir katman olur. Grubun bir katmanını verin.", "layer", t))
    out.append(refused("Etkin katman: servis katmanı", {"operation": "active", "layer": "altlik"}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; etkin katman nesneleri alan bir katman olur.", "layer", t))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "showAll", "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision", t))
    out.append(conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                             step("execute", {"operation": "locked", "layer": "yol", "value": True}, set_result(), {"revision": "changed"}),
                             {"operation": "showAll"}, {"layers": changed(t, "yol", locked=True)}))
    return out


# ── cad.layers.states ────────────────────────────────────────────────────────

def capture(nodes, id_, name, locks, styles):
    """docs/adr/0177 §4: every node in the tree's order with its own visibility; with locks its own lock, with styles a
    layer's style."""
    out = []
    for n, _ in walk(nodes):
        e = {"node": n["id"], "visible": n["visible"]}
        if locks:
            e["locked"] = n["locked"]
        if styles and n["type"] == "layer":
            e["style"] = copy.deepcopy(n["style"])
        out.append(e)
    return {"id": id_, "name": name, "nodes": out}


def apply_state(nodes, state):
    """What applying a state does: the nodes it names that the tree has take its visibility and its lock (when kept),
    the layers its style (when kept); the counts of each and of the nodes the tree lacks."""
    out = copy.deepcopy(nodes)
    vis = lock = sty = missing = 0
    for e in state["nodes"]:
        n = get(out, e["node"])
        if n is None:
            missing += 1
            continue
        if n["visible"] != e["visible"]:
            n["visible"] = e["visible"]
            vis += 1
        if "locked" in e and n["locked"] != e["locked"]:
            n["locked"] = e["locked"]
            lock += 1
        if "style" in e and n["type"] == "layer" and n["style"] != e["style"]:
            n["style"] = copy.deepcopy(e["style"])
            sty += 1
    return out, vis, lock, sty, missing


def styles_only(before, after):
    """The tree after an undo of a state's step: the styles back, the visibility and locks as applied."""
    out = copy.deepcopy(after)
    for n, _ in walk(out):
        n["style"] = copy.deepcopy(get(before, n["id"])["style"])
    return out


def states_layers():
    return [
        node("kadastro", "Kadastro", "group", [node("parsel", "Parsel", style=BLUE), node("bina", "Bina", locked=True)]),
        node("yol", "Yol", visible=False),
        node("gizli", "Gizli"),
    ]


def pafta(nodes):
    """The project's state “Pafta”: the parcels red, the road shown, the hidden layer hidden, the building unlocked,
    and a node the tree no longer has."""
    s = capture(nodes, "durum-1", "Pafta", True, True)
    for e in s["nodes"]:
        if e["node"] == "parsel":
            e["style"] = {**BLUE, "color": "#E5484D"}
        if e["node"] == "yol":
            e["visible"] = True
        if e["node"] == "gizli":
            e["visible"] = False
        if e["node"] == "bina":
            e["locked"] = False
    s["nodes"].append({"node": "silinmis", "visible": True})
    return s


def states_done(state, changed_=True, visible=0, locked_=0, styles=0, missing=0):
    return completed({"state": state, "changed": changed_, "visible": visible, "locked": locked_, "styles": styles, "missing": missing, "revision": "$current"})


def states_cases():
    t = states_layers()
    p = pafta(t)
    out = []
    flags = {"ids": IDS, "dirty": True, "revision": "changed", "canUndo": False}
    new = capture(t, "durum-2", "Çalışma", False, True)
    out.append({"name": "Kaydet: ağacın görünürlüğü ve stilleri; kimliği ilk boş durum-n; proje ayarıdır, geri alma adımı değildir",
                "steps": [step("execute", {"operation": "save", "name": " Çalışma "}, states_done("durum-2"), {**flags, "settings": {"layerStates": [p, new]}})]})
    new = capture(t, "durum-2", "Durum 1", True, False)
    out.append({"name": "Kaydet: adsız “Durum 1”, kilitlerle, stilsiz",
                "steps": [step("execute", {"operation": "save", "locks": True, "styles": False}, states_done("durum-2"), {**flags, "settings": {"layerStates": [p, new]}})]})
    out.append({"name": "Güncelle: ağacın şimdiki hâliyle, sakladıklarıyla",
                "steps": [step("execute", {"operation": "update", "state": "durum-1"}, states_done("durum-1"),
                               {**flags, "settings": {"layerStates": [capture(t, "durum-1", "Pafta", True, True)]}})]})
    out.append({"name": "Güncelle: kilitler bırakılır",
                "steps": [step("execute", {"operation": "update", "state": "durum-1", "locks": False}, states_done("durum-1"),
                               {**flags, "settings": {"layerStates": [capture(t, "durum-1", "Pafta", False, True)]}})]})
    out.append({"name": "Yeniden adlandır",
                "steps": [step("execute", {"operation": "rename", "state": "durum-1", "name": "Pafta 1:1000"}, states_done("durum-1"),
                               {**flags, "settings": {"layerStates": [dict(p, name="Pafta 1:1000")]}})]})
    out.append({"name": "Yeniden adlandır: aynı ad bir şey değiştirmez",
                "steps": [step("execute", {"operation": "rename", "state": "durum-1", "name": "Pafta"}, states_done("durum-1", False),
                               {**unchanged(t), "settings": {"layerStates": [p]}})]})
    # An empty list is not written: the setting reads as absent (null).
    out.append({"name": "Sil", "steps": [step("execute", {"operation": "remove", "state": "durum-1"}, states_done("durum-1"), {**flags, "settings": {"layerStates": None}})]})
    applied, vis, lock, sty, missing = apply_state(t, p)
    out.append({"name": "Uygula: görünürlük ve kilit ağacın kendi değişikliği, stiller tek adımda “Katman durumu: Pafta”; olmayan düğüm sayılır",
                "steps": [step("execute", {"operation": "apply", "state": "durum-1"}, states_done("durum-1", True, vis, lock, sty, missing),
                               {"layers": applied, "canUndo": True, "dirty": True}),
                          step("undo", returns="Katman durumu: Pafta", expect={"layers": styles_only(t, applied)})]})
    out.append({"name": "Uygula: ağaç zaten durumda",
                "steps": [step("execute", {"operation": "apply", "state": "durum-1"}, states_done("durum-1", True, vis, lock, sty, missing), {"layers": applied}),
                          step("execute", {"operation": "apply", "state": "durum-1"}, states_done("durum-1", False, 0, 0, 0, missing), {"layers": applied, "revision": "same"})]})
    out.append({"name": "Plan: uygulamanın sayıları; yazmaz",
                "steps": [step("plan", {"operation": "apply", "state": "durum-1"}, states_done("durum-1", True, vis, lock, sty, missing), unchanged(t))]})
    out.append({"name": "Doğrula: yazmaz", "steps": [step("validate", {"operation": "remove", "state": "durum-1"}, VALID, unchanged(t))]})
    out.append(refused("durumsuz", {"operation": "apply"}, "no_state", "Katman durumunu kimliğiyle verin (state).", "state", t))
    out.append(refused("Yeniden adlandır: adsız", {"operation": "rename", "state": "durum-1"}, "empty_name", "Durumun adını yazın.", "name", t))
    out.append(refused("Kaydet: boş ad", {"operation": "save", "name": "  "}, "empty_name", "Durumun adını yazın.", "name", t))
    out.append(refused("Kaydet: aynı adlı durum", {"operation": "save", "name": "Pafta "}, "duplicate_name",
                       "“Pafta” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın.", "name", t))
    out.append(refused("projede olmayan durum", {"operation": "remove", "state": "durum-9"}, "state_not_found", "“durum-9” kimlikli katman durumu projede yok.", "state", t))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "apply", "state": "durum-1", "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION,
                       "expectedRevision", t))
    out.append(conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                             step("execute", {"operation": "save", "name": "B"}, states_done("durum-2"), {"revision": "changed"}),
                             {"operation": "remove", "state": "durum-1"}, {"settings": {"layerStates": [p, capture(t, "durum-2", "B", False, True)]}}))
    # A second state named as the next free name; the name rule passes spaces at the ends over.
    two = [p, dict(capture(t, "durum-2", "Durum 1", False, False))]
    out.append({"name": "Kaydet: “Durum 1” varsa “Durum 2”; kimliği durum-3",
                "setup": setup(t, layerStates=two),
                "steps": [step("execute", {"operation": "save"}, states_done("durum-3"),
                               {**flags, "settings": {"layerStates": two + [capture(t, "durum-3", "Durum 2", False, True)]}})]})
    out.append({"name": "Yeniden adlandır: başka bir durumun adı",
                "setup": setup(t, layerStates=two),
                "steps": [step("execute", {"operation": "rename", "state": "durum-2", "name": "Pafta"},
                               failed("duplicate_name", "“Pafta” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın.", "name"),
                               {**unchanged(t), "settings": {"layerStates": two}})]})
    return out


def build(command, title, cases, note, setup_doc):
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": command,
        "commandVersion": 1,
        "title": title,
        "note": note,
        "setup": setup_doc,
        "cases": cases,
    }


def files():
    note = "Written by scripts/fixtures/layer_tree_command_cases.py from the contract's rules and docs/adr/0248; do not edit by hand."
    return {
        OUT / "cad.layers.edit.json": build("cad.layers.edit", "Katman düzenle", edit_cases(), note, setup(edit_layers())),
        OUT / "cad.layers.set.json": build("cad.layers.set", "Katman ağacı", set_cases(), note, setup(edit_layers())),
        OUT / "cad.layers.states.json": build("cad.layers.states", "Katman durumları", states_cases(), note,
                                              setup(states_layers(), layerStates=[pafta(states_layers())])),
    }


def main() -> int:
    bad = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad += 1
            else:
                print(f"{path.name} güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
