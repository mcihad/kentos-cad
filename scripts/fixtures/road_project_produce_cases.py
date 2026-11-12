#!/usr/bin/env python3
"""The shared cases of `road.project.produce` (Tariften üret; plan/map-cad.md MAP-20, docs/adr/3240): a recipe's
drawings drawn in another project.

The project is road_terrain_zeroline_cases.py's Yol project (TUREF / TM30, its surfaces Düz t-5, Teras t-6, Set t-9,
Tümsek t-10 …, the layer `sifir`); its Harita project, without Teras and Set, for one case. The recipes are İş grafiği's graphs (road.project.graph v1's answer as
Tarifi kaydet… writes it, docs/adr/1940) of another project, written here by hand under fixtures/recipes/v1: its
surfaces have their own ids there (a-5 Düz, a-10 Tümsek, a-6 Teras, a-9 Set) and the names this project has. The
rules, by hand:

- a text of a drawing's input that is the id of one of the recipe's sources is that source's name; a layer's id
  (`layerId`) stays; the command finds the name here (case aside);
- each drawing is the command's own: a zero line's lines are road_terrain_zeroline_cases.py's, one object each (Düz
  under Tümsek: one closed line; Teras under Set: two open lines);
- the drawings are kept as this project's, numbered cz-1, cz-2 … after those it has;
- a drawing this project has (the same command, main source and layer) is left (`exists`);
- named drawings are all drawn or none, a command's refusal with the recipe's drawing named; every drawing asked, one
  the command refuses here is left with its words (`refused`);
- a recipe saved in a project of another system is warned; its inputs are taken as they are;
- the file, the recipe and the names asked are checked before anything is drawn;
- a drawing's layer (docs/adr/3640): the one `layers` gives for its recipe layer; else, the recipe keeping the layer's
  definition, the one layer here of that name folded as Katmanları çevir folds it (I → ı, İ → i, then lower case;
  two of them: asked, the drawing left or, named, refused), or one added last with the recipe's colour, line type and
  weight, said as a warning and taken away again when no drawing went on it; a recipe from before, the layer's id. The
  answer lists each recipe layer the produced drawings went on, in the order they first did, and how it was found;
- a recipe layer keeping its groups, whole style and line type (docs/adr/4040): a layer added goes in its groups, each
  the first group of its name (folded) among its parent's, else added last in it; it takes the whole style; the
  project line type the style names is this project's of that name (letters' case aside), else added from the
  recipe's definition, else left out of the style (the built-in type stays); the answer says the groups it went in
  (their names here), those added and the line type added. A layer taken away again takes the groups added for it
  that hold nothing then and the line type added for it that no layer names. A definition whose style's colour, line
  type or weight is not its own, with an empty group name, or with a line type its style does not name or that does
  not hold, is refused;
- a recipe layer keeping its composition, type and Yazdır (docs/adr/4440): a layer added takes its opacity and blending
  mode, its type and, when it is not printed, Yazdır off, said after the rest (“Tarifteki gibi saydamlığı %40, karışım
  kipi Çoğalt, türü Çizgi ve yazdırılmaz.”: the transparency is 1 − the opacity in percent); a layer found by its name
  keeps its own. An object a drawing writes that its layer's type does not hold (docs/adr/0622 §5: a point layer holds
  points, a line layer lines, polylines, arcs, curves, circles and ellipses; a zero line's lines are polylines) refuses
  the produce, the plan too, whichever way the layer was found. A composition with a mask, one that says nothing, or an
  opacity outside 0–1 is refused;
- a recipe layer keeping its fields, the lists they link and its subtypes (docs/adr/4860): a layer added takes them;
  each list a field links is this project's list of its name (letters' case aside) when that one holds the same (its
  kind, values, range and unit), the field linked to it; else the recipe's is added: under its own name and id when
  none of its name is here (the id made of the name when the recipe's is taken), under the first free name
  `<name>_1`, `<name>_2` … with an id made of that name when this project's of its name holds other values (it is left
  as it is). Said after the rest: “Alanları (3) ve alt türleri tarifteki gibi.” and how each list went; the answer
  names the lists added. A layer found by its name keeps its own fields; the recipe's fields it lacks and those it
  holds as another kind are said (`recipe_layer_fields`). A layer taken away again takes the lists added for it that no
  field links then. A field carrying a bound table's column, a field linking a list the recipe does not define or
  whose values are not its list's, subtypes that do not fit the fields and two fields of one name are refused.

No KentOS code.

    python3 scripts/fixtures/road_project_produce_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_zeroline_cases as zc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.project.produce.json"
RECIPES = ROOT / "fixtures/recipes/v1"
ANY = "$any"
W = 1e-6
UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
SAME = {"revision": "same"}
ZERO = "road.terrains.zeroline"
WORDS = {ZERO: "Sıfır hattı"}
# The recipe's project's surfaces: their ids there, the names this project has.
SOURCES = [("a-5", "Düz"), ("a-10", "Tümsek"), ("a-6", "Teras"), ("a-9", "Set")]
# This project's surfaces by name (road_terrain_zeroline_cases.py's).
HERE = {n.lower(): i for i, n in zc.NAMES.items()}
EXISTS = "Projede bu çizim var (aynı komut, ana kaynak ve katman); Yeniden üret ile güncellenir."
NOT_ROAD = {"status": "failed", "error": {"code": "not_a_road_project", "message": "Proje ağacı yol ve harita projesinde "
                                          "(KentOS Yol, KentOS Harita) olur; bu çizim ikisinden biri değil."}}


def drawing(id_, inp, command=ZERO, version=1, objects=1, layer=None):
    return (id_, command, version, inp, objects, layer)


TWO = [drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "sifir"}),
       drawing("cz-2", {"base": "a-6", "comparison": "Set", "layerId": "sifir"}, objects=2)]


def walk(layers, out=None):
    out = {} if out is None else out
    for n in layers:
        out[n["id"]] = n
        walk(n.get("children", []), out)
    return out


# This project's layers by id (road_terrain_zeroline_cases.py's setup).
LAYERS = walk(zc.setup(work="road")["layers"])


def ldef(id_, name, color="#E5484D", line_type="continuous", weight=0.5):
    """Docs/adr/3640: a recipe layer's definition, as İş grafiği writes it."""
    return {"id": id_, "name": name, "color": color, "lineType": line_type, "lineWeight": weight}


# Docs/adr/4040: a project line type of the recipe's project (millimetres on paper) and a layer's whole style naming it
# (its nearest built-in type dash-dot), with a fill, a point and a label.
SINIR = {"name": "SINIR", "description": "Kazı sınırı", "elements": [{"length": 4.0}, {"length": -1.0}, {"length": 0.5},
                                                                    {"length": -1.0}], "unit": "paper"}
STYLE = {"color": "#2A9D8F", "lineType": "dashdot", "lineWeight": 0.35, "linePattern": "SINIR", "fill": "#2A9D8F",
         "point": {"symbol": "cross", "size": 6.0}, "label": {"placement": "along", "size": 10.0}}


def sdef(id_, name, group=(), style=None, line_type=SINIR):
    """Docs/adr/4040: a recipe layer's definition with its groups, whole style and line type's definition."""
    st = copy.deepcopy(STYLE if style is None else style)
    out = ldef(id_, name, color=st["color"], line_type=st["lineType"], weight=st["lineWeight"])
    if group:
        out["group"] = list(group)
    out["style"] = st
    if line_type is not None:
        out["lineTypeDef"] = copy.deepcopy(line_type)
    return out


K7 = ldef("k-7", "Sıfır hattı")
K9 = ldef("k-9", "Kazı sınırı", color="#2A9D8F", line_type="dashed", weight=0.35)
K11 = ldef("k-11", "Ova sınırı", color="#264653", line_type="dotted", weight=0.18)
TWO_K = [drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "k-7"}, layer=K7),
         drawing("cz-2", {"base": "a-6", "comparison": "Set", "layerId": "k-7"}, objects=2, layer=K7)]
TWO_NEW = [drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "k-9"}, layer=K9),
           drawing("cz-2", {"base": "a-6", "comparison": "Set", "layerId": "k-9"}, objects=2, layer=K9)]
# Docs/adr/4040: “Kazı sınırı” in the group Yol (written YOL: folded, the setup's Yol) › Toprak işleri; in two groups
# this project lacks; a style naming a line type the recipe does not define.
K12 = sdef("k-12", "Kazı sınırı", group=("YOL", "Toprak işleri"))
K13 = sdef("k-13", "Kazı sınırı", group=("Toprak", "Sınırlar"))
K14 = sdef("k-14", "Kazı sınırı", style=dict(STYLE, linePattern="YOKTUR"), line_type=None)
# Docs/adr/4440: a layer 40 % transparent in the multiply mode, a line layer not printed (its style names no project
# line type); the same under the zero line's name (found here: its own look stays); a point layer; three inconsistent.
PLAIN = {k: v for k, v in STYLE.items() if k != "linePattern"}
LOOK = {"compose": {"opacity": 0.6, "blend": "multiply"}, "geometry": "line", "plot": False}
K15 = dict(sdef("k-15", "Kazı sınırı", style=PLAIN, line_type=None), **copy.deepcopy(LOOK))
K16 = dict(sdef("k-7", "Sıfır hattı", style=PLAIN, line_type=None), **copy.deepcopy(LOOK))
K17 = dict(sdef("k-17", "Kazı noktaları", style=PLAIN, line_type=None), geometry="point")
# Docs/adr/4860: a layer's fields (a name, a kind linked to the recipe project's list Sınır türü, a depth in metres),
# the list and its subtypes by the kind (a cut's depth required); the same under the zero line's name (found here: its
# own fields stay); five inconsistent.
SINIR_TURU = {"id": "sinir-turu", "name": "Sınır türü", "kind": "text",
              "values": [{"code": "K", "label": "Kazı"}, {"code": "D", "label": "Dolgu"}]}
FIELDS = [{"name": "Ad", "kind": "text", "length": 40},
          {"name": "Tür", "kind": "text", "values": copy.deepcopy(SINIR_TURU["values"]), "domain": "sinir-turu"},
          {"name": "Derinlik", "kind": "decimal", "scale": 2, "unit": "m"}]
SUBTYPES = {"field": "Tür", "types": [{"code": "K", "label": "Kazı sınırı", "fields": [{"field": "Derinlik",
                                                                                      "required": True}]},
                                      {"code": "D", "label": "Dolgu sınırı"}]}
K20 = dict(sdef("k-20", "Kazı sınırı", style=PLAIN, line_type=None), fields=copy.deepcopy(FIELDS),
           domains=[copy.deepcopy(SINIR_TURU)], subtypes=copy.deepcopy(SUBTYPES))
K21 = dict(K20, id="k-7", name="Sıfır hattı")
# Docs/adr/5240: the layer's value combination (the kind and a ground field with its own values), record rule, index
# and form (a draft bound to the name and the kind by their keys); the same under the zero line's name.
ZEMIN = {"name": "Zemin", "kind": "text", "values": [{"code": "T", "label": "Toprak"}, {"code": "Y", "label": "Kaya"}]}
SHAPED_FIELDS = copy.deepcopy(FIELDS) + [copy.deepcopy(ZEMIN)]
GROUP = {"name": "Tür ve zemin", "fields": ["Tür", "Zemin"], "restrictive": True,
         "rows": [{"cells": [{"kind": "code", "code": "K"}, {"kind": "any"}]},
                  {"cells": [{"kind": "code", "code": "D"}, {"kind": "code", "code": "T"}]}]}
RULE = {"id": "derinlik-arti", "name": "Derinlik artı", "rule": "Derinlik boş veya Derinlik > 0",
        "message": "Derinlik sıfırdan büyük olmalı.", "fields": ["Derinlik"]}
INDEX = {"name": "Ad", "fields": ["Ad"], "unique": True}
FORM = {"id": "sinir-formu", "name": "Sınır formu",
        "draft": {"revision": 1, "items": [{"type": "field", "id": "ad", "field": "Ad", "required": True},
                                           {"type": "field", "id": "tur", "field": "Tür"}]}}
K22 = dict(K20, fields=copy.deepcopy(SHAPED_FIELDS), contingents=[copy.deepcopy(GROUP)], rules=[copy.deepcopy(RULE)],
           indexes=[copy.deepcopy(INDEX)], forms=[copy.deepcopy(FORM)])
K23 = dict(K22, id="k-7", name="Sıfır hattı")


def on_layer(drawings, layer):
    """The drawings with their layer `layer` (its id in their inputs, its definition kept)."""
    out = []
    for id_, command, version, inp, objects, _ in drawings:
        out.append((id_, command, version, dict(inp, layerId=layer["id"]), objects, layer))
    return out


def recipe(drawings, sources=SOURCES, epsg="EPSG:5254", system="TUREF / TM30"):
    """The graph as Tarifi kaydet… writes it: the recipe's project's surfaces, then its drawings with their recipes and
    their sources' edges (the main one first, its stamp then and now)."""
    nodes = [{"kind": "surface", "id": sid, "name": name, "state": "current"} for sid, name in sources]
    edges = []
    for id_, command, version, inp, objects, layer in drawings:
        k = len(nodes)
        nodes.append({"kind": "drawing", "id": id_, "name": WORDS.get(command, "Yol çizimi"), "state": "current",
                      "recipe": {"command": command, "version": version, "input": inp, "written": objects,
                                 "objects": objects, "digest": "0123456789abcdef", "made": 1, "revision": "7"}})
        if layer is not None:
            nodes[-1]["recipe"]["layer"] = layer
        for field, main in (("base", True), ("comparison", False)):
            at = next((i for i, n in enumerate(nodes[:len(sources)]) if n["id"] == inp.get(field)), None)
            if at is not None:
                edges.append({"from": at, "to": k, "kind": "source", **({"main": True} if main else {}),
                              "kept": "0123456789abcdef", "now": "0123456789abcdef"})
    out = {"nodes": nodes, "edges": edges, "drawings": len(drawings), "outdated": 0, "sheets": 0, "blocked": 0}
    if epsg is not None:
        out.update({"epsg": epsg, "system": system})
    out.update({"deep": False, "revision": "9"})
    return out


# The recipe files: name → graph (or text: a file that is not a recipe).
FILES = {
    "sifir-hatlari.json": recipe(TWO),
    "baska-sistem.json": recipe(TWO, epsg="EPSG:5256", system="TUREF / TM36"),
    "yerel-proje.json": recipe(TWO, epsg=None),
    # The recipe's ids are this project's ids of other surfaces: the names decide.
    "kimlik-cakisan.json": recipe([drawing("cz-1", {"base": "t-5", "comparison": "t-9", "layerId": "sifir"}, objects=2)],
                                  sources=[("t-5", "Teras"), ("t-9", "Set")]),
    "buyuk-harfli.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "sifir"})],
                                sources=[("a-5", "DÜZ"), ("a-10", "tümsek")]),
    "eksik-kaynakli.json": recipe(TWO + [drawing("cz-3", {"base": "a-1", "comparison": "a-5", "layerId": "sifir"})],
                                  sources=SOURCES + [("a-1", "Ova")]),
    "eksik-katmanli.json": recipe([TWO[0], drawing("cz-4", {"base": "a-6", "comparison": "a-9", "layerId": "sifir-a"},
                                                   objects=2)]),
    "baska-katmanli.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "drenaj"})]),
    "gizli-katmanli.json": recipe([drawing("cz-1", {"base": "a-6", "comparison": "a-9", "layerId": "gizli"}, objects=2)]),
    "kilitli-katmanli.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "kilitli"})]),
    "grup-katmanli.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "grup"})]),
    "cizmeyen-komut.json": recipe([drawing("cz-1", {"surface": "a-5"}, command="road.terrains.query")]),
    "baska-surum.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "sifir"}, version=2)]),
    "cizimsiz.json": recipe([]),
    "tarif-degil.json": {"format": "kentos.field-package", "version": 1},
    # Docs/adr/3640: recipes keeping their layers' definitions.
    "katman-adli.json": recipe(TWO_K),
    "katman-buyuk-harfli.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "k-7"},
                                                layer=dict(K7, name="SIFIR HATTI"))]),
    "katman-kimligi-cakisan.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "drenaj"},
                                                   layer=dict(K7, id="drenaj"))]),
    "yeni-katmanli.json": recipe(TWO_NEW),
    "yeni-katmanli-eksik.json": recipe(TWO_K + [drawing("cz-3", {"base": "a-1", "comparison": "a-5", "layerId": "k-11"},
                                                        layer=K11)], sources=SOURCES + [("a-1", "Ova")]),
    "tanimi-tutmayan.json": recipe([drawing("cz-1", {"base": "a-5", "comparison": "a-10", "layerId": "k-7"},
                                            layer=dict(K7, id="k-8"))]),
    # Docs/adr/4040: recipes keeping their layers' groups, whole styles and line types.
    "grup-stilli.json": recipe(on_layer(TWO, K12)),
    "yeni-gruplu.json": recipe(on_layer(TWO, K13)),
    "tanimsiz-cizgi-turlu.json": recipe(on_layer(TWO[:1], K14)),
    "yeni-gruplu-eksik.json": recipe(TWO_K + [drawing("cz-3", {"base": "a-1", "comparison": "a-5", "layerId": "k-13"},
                                                      layer=K13)], sources=SOURCES + [("a-1", "Ova")]),
    "stili-tutmayan.json": recipe(on_layer(TWO[:1], dict(K12, color="#000000"))),
    "bos-gruplu.json": recipe(on_layer(TWO[:1], dict(K12, group=["Yol", " "]))),
    "adlanmayan-cizgi-turlu.json": recipe(on_layer(TWO[:1], dict(K12, lineTypeDef=dict(SINIR, name="KESIK")))),
    "bosluklu-cizgi-turlu.json": recipe(on_layer(TWO[:1], dict(K12, lineTypeDef=dict(SINIR, name=" SINIR")))),
    # Docs/adr/4440: recipes keeping their layers' compositions, types and Yazdır.
    "saydam-katmanli.json": recipe(on_layer(TWO, K15)),
    "saydam-adli.json": recipe(on_layer(TWO, K16)),
    "nokta-katmanli.json": recipe(on_layer(TWO[:1], K17)),
    "maskeli.json": recipe(on_layer(TWO[:1], dict(K15, compose={"opacity": 0.6, "mask": {
        "labels": True, "size": 1.0, "targets": ["drenaj"]}}))),
    "bos-cizimli.json": recipe(on_layer(TWO[:1], dict(K15, compose={}))),
    "saydamligi-tutmayan.json": recipe(on_layer(TWO[:1], dict(K15, compose={"opacity": 1.5}))),
    # Docs/adr/4860: recipes keeping their layers' fields, lists and subtypes.
    "alanli-katmanli.json": recipe(on_layer(TWO, K20)),
    "alanli-adli.json": recipe(on_layer(TWO, K21)),
    "alanli-eksik.json": recipe(TWO_K + [drawing("cz-3", {"base": "a-1", "comparison": "a-5", "layerId": "k-20"},
                                                 layer=K20)], sources=SOURCES + [("a-1", "Ova")]),
    "bagli-sutunlu.json": recipe(on_layer(TWO[:1], dict(K20, fields=FIELDS + [
        {"name": "Kayıt günü", "kind": "date", "column": {"made": "now()"}}]))),
    "listesiz-alanli.json": recipe(on_layer(TWO[:1], {k: v for k, v in K20.items() if k != "domains"})),
    "liste-tutmayan.json": recipe(on_layer(TWO[:1], dict(K20, fields=[FIELDS[0], dict(FIELDS[1], values=[
        {"code": "K", "label": "Kazı"}]), FIELDS[2]]))),
    "alt-turu-tutmayan.json": recipe(on_layer(TWO[:1], dict(K20, subtypes=dict(SUBTYPES, field="Yok")))),
    "iki-ad-alanli.json": recipe(on_layer(TWO[:1], dict(K20, fields=[FIELDS[0]] + FIELDS))),
    # Docs/adr/5240: recipes keeping their layers' value combinations, record rules, indexes and forms.
    "tanimli-katmanli.json": recipe(on_layer(TWO, K22)),
    "tanimli-adli.json": recipe(on_layer(TWO, K23)),
    "tanimli-eksik.json": recipe(TWO_K + [drawing("cz-3", {"base": "a-1", "comparison": "a-5", "layerId": "k-20"},
                                                  layer=K22)], sources=SOURCES + [("a-1", "Ova")]),
    "bilesimi-tutmayan.json": recipe(on_layer(TWO[:1], dict(K22, contingents=[dict(GROUP, fields=["Tür", "Yok"])]))),
    "kurali-tutmayan.json": recipe(on_layer(TWO[:1], dict(K22, rules=[RULE, RULE]))),
    "dizini-tutmayan.json": recipe(on_layer(TWO[:1], dict(K22, indexes=[dict(INDEX, fields=[])]))),
    "formu-tutmayan.json": recipe(on_layer(TWO[:1], dict(K22, forms=[dict(FORM, draft={"revision": 1, "items": [
        {"type": "field", "id": "yok", "field": "Yok"}]})]))),
    "iki-formlu.json": recipe(on_layer(TWO[:1], dict(K22, forms=[FORM, dict(FORM, name="İkinci")]))),
}


def fx(name):
    return f"$fixture:recipes/v1/{name}"


def lines(base, comparison):
    """How many lines the zero line of `comparison` over `base` has here (road_terrain_zeroline_cases.py's)."""
    return len(zc.answer({"base": base, "comparison": comparison})["lines"])


def produced(from_, drawing_id, base, comparison, written=True):
    sid = HERE[base.lower()]
    n = lines(base, comparison)
    return {"from": from_, "drawing": drawing_id, "command": ZERO,
            "source": {"kind": "surface", "id": sid, "name": zc.NAMES[sid]}, "uids": [ANY] * n if written else []}


def done(out, warnings=()):
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def failed(code, message, path=None):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    return {"status": "failed", "error": err}


def step(inp, result, op="execute", expect=None):
    s = {"op": op, "input": inp, "result": result}
    s["expect"] = expect if expect is not None else (STEPPED if op == "execute" and result["status"] == "completed"
                                                     else UNCHANGED)
    if result["status"] == "completed":
        s["within"] = W
    return s


def case(name, *steps, setup=None, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = list(steps)
    if setup is not None:
        c["setup"] = setup
    return c


def both(written=True, first="cz-1", second="cz-2"):
    return [produced("cz-1", first, "Düz", "Tümsek", written), produced("cz-2", second, "Teras", "Set", written)]


def went(from_, to=None, how="id", name=None, to_name=None):
    """Docs/adr/3640: a recipe layer the produced drawings went on: its id and name in the recipe (a recipe from before:
    the layer's name here), the layer here (its name: the setup's, else `to_name`) and how it was found."""
    to = from_ if to is None else to
    here = to_name if to_name is not None else LAYERS[to]["name"]
    return {"from": from_, "name": here if name is None else name, "to": to, "toName": here, "how": how}


SIFIR = [went("sifir")]
CREATED = {"code": "recipe_layer_created", "message": "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki "
           "tanımıyla (renk, çizgi türü, kalınlık) eklendi.", "path": "layers"}


def doubled():
    """The project with a second layer of the zero line's name (folded the same): which one is asked."""
    s = zc.setup(work="road")
    twin = copy.deepcopy(next(n for n in s["layers"] if n["id"] == "sifir"))
    twin.update({"id": "sifir-2", "name": "SIFIR HATTI"})
    s["layers"].append(twin)
    return s


def cases():
    out = []
    path = fx("sifir-hatlari.json")
    two = {"produced": both(), "layers": SIFIR, "revision": "$current"}
    out.append(case("Tarifin iki sıfır hattı bu projede: kaynaklar adlarıyla", step({"path": path}, done(two)),
                    note="a-5 Düz → t-5, a-10 Tümsek → t-10 (bir kapalı çizgi); a-6 Teras → t-6, Set adıyla (iki açık "
                         "çizgi)"))
    out.append(case("Plan yazmaz", step({"path": path}, done({"produced": both(False), "layers": SIFIR,
                                                             "revision": "$current"}), op="plan")))
    out.append(case("Doğrulama yazmaz", step({"path": path}, done(None), op="validate")))
    out.append(case("Yalnız biri", step({"path": path, "drawings": ["cz-2"]},
                                       done({"produced": [produced("cz-2", "cz-1", "Teras", "Set")], "layers": SIFIR,
                                             "revision": "$current"}))))
    left_both = [{"drawing": "cz-1", "why": "exists", "message": EXISTS},
                 {"drawing": "cz-2", "why": "exists", "message": EXISTS}]
    out.append(case("İkinci kez: ikisi de projede var, hiçbir şey yazılmaz", step({"path": path}, done(two)),
                    step({"path": path}, done({"produced": [], "left": left_both, "revision": "$current"}),
                         expect=SAME)))
    direct = {"op": "execute", "command": ZERO, "input": {"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"},
              "result": {"status": "completed", "output": ANY, "warnings": []}}
    out.append(case("Projede aynı çizim var: bırakılır, öbürü üretilir", direct,
                    step({"path": path}, done({"produced": [produced("cz-2", "cz-2", "Teras", "Set")],
                                               "left": [left_both[0]], "layers": SIFIR, "revision": "$current"}))))
    elsewhere = dict(direct, input={"base": "Düz", "comparison": "Tümsek", "layerId": "drenaj"})
    out.append(case("Başka katmandaki aynı çizim: yine üretilir", elsewhere,
                    step({"path": path}, done({"produced": both(first="cz-2", second="cz-3"), "layers": SIFIR,
                                               "revision": "$current"}))))
    out.append(case("Tarifin katmanı başka: o katmana", step({"path": fx("baska-katmanli.json")},
                                                         done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                                                               "layers": [went("drenaj")], "revision": "$current"}))))
    out.append(case("Harita projesinde de", step({"path": path, "drawings": ["cz-1"]},
                                                 done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                                                       "layers": SIFIR, "revision": "$current"})), setup="$harita"))
    other = {"code": "recipe_other_system", "message": "Tarif TUREF / TM36 (EPSG:5256) projesinde kaydedilmiş; bu proje "
             "TUREF / TM30 (EPSG:5254). Girdiler aynen alınır.", "path": "path"}
    out.append(case("Başka sistemde kaydedilmiş tarif: uyarıyla", step({"path": fx("baska-sistem.json")},
                                                                      done(two, [other]))))
    local = dict(other, message="Tarif yerel (koordinat sistemi yok) projesinde kaydedilmiş; bu proje TUREF / TM30 "
                                "(EPSG:5254). Girdiler aynen alınır.")
    out.append(case("Yerel projede kaydedilmiş tarif: uyarıyla", step({"path": fx("yerel-proje.json")},
                                                                     done(two, [local]))))
    out.append(case("Tarifin kimlikleri bu projede başka yüzeylerin: adlar seçer",
                    step({"path": fx("kimlik-cakisan.json")},
                         done({"produced": [produced("cz-1", "cz-1", "Teras", "Set")], "layers": SIFIR,
                               "revision": "$current"})),
                    note="tarifte t-5 Teras, t-9 Set; burada t-5 Düz: kimlikle çizilseydi Düz ile Set"))
    out.append(case("Adlar büyük küçük harf farkıyla", step({"path": fx("buyuk-harfli.json")},
                                                         done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                                                               "layers": SIFIR, "revision": "$current"}))))
    ova = "“Ova” kimlikli ya da adlı yüzey projede yok."
    out.append(case("Bütün çizimler: projede olmayan yüzeyin çizimi bırakılır",
                    step({"path": fx("eksik-kaynakli.json")},
                         done({"produced": both(), "left": [{"drawing": "cz-3", "why": "refused", "message": ova}],
                               "layers": SIFIR, "revision": "$current"}))))
    out.append(case("Adıyla istenen çizimin yüzeyi projede yok: ret",
                    step({"path": fx("eksik-kaynakli.json"), "drawings": ["cz-1", "cz-3"]},
                         failed("unknown_terrain", f"Tarifin “cz-3” çizimi (Sıfır hattı): {ova}", "base"))))
    out.append(case("Adıyla istenen ikinci çizimin katmanı yok: hiçbiri yazılmaz",
                    step({"path": fx("eksik-katmanli.json"), "drawings": ["cz-1", "cz-4"]},
                         failed("layer_not_found", "Tarifin “cz-4” çizimi (Sıfır hattı): “sifir-a” kimlikli katman "
                                "çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))))
    out.append(case("Bütün çizimler: katmanı olmayan bırakılır",
                    step({"path": fx("eksik-katmanli.json")},
                         done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                               "left": [{"drawing": "cz-4", "why": "refused", "message": "“sifir-a” kimlikli katman "
                                         "çizimde yok. Var olan bir katmanın kimliğini verin."}],
                               "layers": SIFIR, "revision": "$current"}))))
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    out.append(case("Gizli katmana: komutun uyarısıyla", step({"path": fx("gizli-katmanli.json")},
                                                          done({"produced": [produced("cz-1", "cz-1", "Teras", "Set")],
                                                                "layers": [went("gizli")], "revision": "$current"},
                                                               [hidden]))))
    out.append(case("Adıyla istenen çizimin katmanı kilitli: ret",
                    step({"path": fx("kilitli-katmanli.json"), "drawings": ["cz-1"]},
                         failed("layer_locked", "Tarifin “cz-1” çizimi (Sıfır hattı): “Kilitli” katmanı kilitli. Kilidi "
                                "Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"))))
    out.append(case("Adıyla istenen çizimin katmanı bir grup: ret",
                    step({"path": fx("grup-katmanli.json"), "drawings": ["cz-1"]}, failed("not_a_layer", ANY, "layerId"))))
    out += layer_cases()
    out += group_cases()
    out += look_cases()
    out += fields_cases()
    out += shaped_cases()
    # The file and the recipe.
    out.append(case("Tarifte olmayan çizim", step({"path": path, "drawings": ["cz-9"]},
                                                 failed("unknown_drawing", "Tarifte “cz-9” kimlikli çizim yok.",
                                                        "drawings"))))
    out.append(case("Göreli yol", step({"path": "tarifler/sifir-hatlari.json"},
                                       failed("invalid_file_path", "Tarifin yolu boş ya da göreli; mutlak bir .json "
                                              "yolu verin.", "path"))))
    out.append(case(".json olmayan dosya", step({"path": "$fixture:asbuilt/v1/files/saha-paketi.zip"},
                                               failed("invalid_file_path", "“$fixture:asbuilt/v1/files/saha-paketi.zip” "
                                                      "bir .json dosyası değil (İş grafiği'nin tarifi).", "path"))))
    out.append(case("Olmayan dosya", step({"path": fx("yok.json")},
                                          failed("file_not_found", f"“{fx('yok.json')}” tarifi yok; İş grafiği'nin "
                                                 "Tarifi kaydet… ile yazdığı .json dosyasını seçin.", "path"))))
    out.append(case("Tarif olmayan JSON", step({"path": fx("tarif-degil.json")}, failed("invalid_project_recipe", ANY, "path"))))
    out.append(case("Çizimi olmayan tarif", step({"path": fx("cizimsiz.json")},
                                                failed("invalid_project_recipe", "“cizimsiz.json” tarifinde üretilecek çizim "
                                                       "yok.", "path"))))
    out.append(case("Çizmeyen komut", step({"path": fx("cizmeyen-komut.json")},
                                          failed("invalid_project_recipe", "Tarifteki “cz-1” çiziminin komutu "
                                                 "(road.terrains.query) çizen bir komut değil.", "path"))))
    out.append(case("Başka sürümün tarifi", step({"path": fx("baska-surum.json")},
                                                failed("invalid_project_recipe", "Tarifteki “cz-1” çizimi road.terrains.zeroline "
                                                       "v2 ile; bu KentOS v1 ile çizer.", "path"))))
    out.append(case("Mimari proje", step({"path": path}, NOT_ROAD), setup="$mimari"))
    out.append(case("Geçersiz beklenen sürüm", step({"path": path, "expectedRevision": "on"}, zc.BAD_REVISION)))
    out.append(case("Eski sürüme göre hazırlanmış", step({"path": path, "expectedRevision": "41"}, zc.CONFLICT)))
    return out


def layer_cases():
    """Docs/adr/3640: the recipe's layers by their definitions: by name, added, asked, given."""
    out = []
    named = fx("katman-adli.json")
    by_name = [went("k-7", "sifir", "name", "Sıfır hattı")]
    out.append(case("Katman adıyla: tarifin kimliği başka", step({"path": named},
                                                              done({"produced": both(), "layers": by_name,
                                                                    "revision": "$current"})),
                    note="tarifte k-7 “Sıfır hattı”; burada sifir"))
    out.append(case("Katman adı Türkçe büyük harflerle", step({"path": fx("katman-buyuk-harfli.json")},
                                                           done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                                                                 "layers": [went("k-7", "sifir", "name", "SIFIR HATTI")],
                                                                 "revision": "$current"})),
                    note="“SIFIR HATTI” katlanınca “sıfır hattı” (I → ı)"))
    out.append(case("Katmanın kimliği bu projede başka katmanın: ad seçer",
                    step({"path": fx("katman-kimligi-cakisan.json")},
                         done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                               "layers": [went("drenaj", "sifir", "name", "Sıfır hattı")], "revision": "$current"})),
                    note="tarifte drenaj “Sıfır hattı”; burada drenaj Drenaj: kimlikle çizilseydi Drenaj'a"))
    new = fx("yeni-katmanli.json")
    created = [went("k-9", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı")]
    left_both = [{"drawing": "cz-1", "why": "exists", "message": EXISTS},
                 {"drawing": "cz-2", "why": "exists", "message": EXISTS}]
    out.append(case("Projede olmayan katman tarifteki tanımıyla eklenir; ikinci kez adıyla bulunur",
                    step({"path": new}, done({"produced": both(), "layers": created, "revision": "$current"},
                                             [CREATED])),
                    step({"path": new}, done({"produced": [], "left": left_both, "revision": "$current"}),
                         expect=SAME)))
    out.append(case("Plan eklenecek katmanı söyler, yazmaz",
                    step({"path": new}, done({"produced": both(False), "layers": created, "revision": "$current"},
                                             [CREATED]), op="plan")))
    ova = "“Ova” kimlikli ya da adlı yüzey projede yok."
    out.append(case("Bütün çizimler: eklenen katmana çizilemeyen çizim bırakılır, katman geri alınır",
                    step({"path": fx("yeni-katmanli-eksik.json")},
                         done({"produced": both(), "left": [{"drawing": "cz-3", "why": "refused", "message": ova}],
                               "layers": by_name, "revision": "$current"}))))
    out.append(case("Adıyla istenen, eklenecek katmanlı çizimin yüzeyi yok: ret, katman da eklenmez",
                    step({"path": fx("yeni-katmanli-eksik.json"), "drawings": ["cz-3"]},
                         failed("unknown_terrain", f"Tarifin “cz-3” çizimi (Sıfır hattı): {ova}", "base"))))
    twice = ("Projede “Sıfır hattı” adlı 2 katman var (sifir, sifir-2); tarifin katmanının hangisi olduğunu layers ile "
             "verin.")
    out.append(case("Aynı adlı iki katman: bütün çizimlerde bırakılır",
                    step({"path": named}, done({"produced": [], "left": [
                        {"drawing": "cz-1", "why": "refused", "message": twice},
                        {"drawing": "cz-2", "why": "refused", "message": twice}], "revision": "$current"}),
                         expect=SAME), setup="$cift"))
    out.append(case("Aynı adlı iki katman, adıyla istenen çizim: ret",
                    step({"path": named, "drawings": ["cz-1"]},
                         failed("ambiguous_recipe_layer", f"Tarifin “cz-1” çizimi (Sıfır hattı): {twice}", "layers")),
                    setup="$cift"))
    out.append(case("Aynı adlı iki katman: layers ile verilen",
                    step({"path": named, "layers": {"k-7": "sifir-2"}},
                         done({"produced": both(), "layers": [went("k-7", "sifir-2", "given", "Sıfır hattı",
                                                                   to_name="SIFIR HATTI")],
                               "revision": "$current"})), setup="$cift"))
    out.append(case("Verilen katman adıyla bulunandan önce gelir",
                    step({"path": named, "layers": {"k-7": "drenaj"}},
                         done({"produced": both(), "layers": [went("k-7", "drenaj", "given", "Sıfır hattı")],
                               "revision": "$current"}))))
    out.append(case("layers'ta tarifte olmayan katman",
                    step({"path": named, "layers": {"zz": "sifir"}},
                         failed("unknown_recipe_layer", "Tarifte “zz” kimlikli katmana çizilen çizim yok; layers'ın "
                                "anahtarı tarifin bir katmanı olmalı.", "layers"))))
    out.append(case("layers'ta projede olmayan katman",
                    step({"path": named, "layers": {"k-7": "yok"}},
                         failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini "
                                "verin.", "layers.k-7"))))
    out.append(case("layers'ta kilitli katman",
                    step({"path": named, "layers": {"k-7": "kilitli"}},
                         failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka "
                                "bir katmanı etkinleştirin.", "layers.k-7"))))
    out.append(case("Tarifin katman tanımı girdisinin katmanı değil",
                    step({"path": fx("tanimi-tutmayan.json")},
                         failed("invalid_project_recipe", "Tarifteki “cz-1” çiziminin katman tanımı (k-8) girdisinin "
                                "katmanı (k-7) değil.", "path"))))
    return out


# Docs/adr/4040: a new node's look and flags (a group's style is the default), and this project's own line type of the
# recipe's line type's name (another case, another pattern).
DEFAULT = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}
HERE_SINIR = {"name": "Sinir", "description": "Bu projenin sınırı", "elements": [{"length": 6.0}, {"length": -2.0}],
              "unit": "paper"}


def node(name, style, kind="layer", children=()):
    return {"id": ANY, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": copy.deepcopy(style), "children": list(children)}


def tree_with(added, into=None, setup=None):
    """The setup's layer tree with `added` last in the group `into` (none: last at the top)."""
    layers = copy.deepcopy((setup or zc.setup(work="road"))["layers"])
    if into is None:
        layers.append(added)
    else:
        walk(layers)[into]["children"].append(added)
    return layers


def own_line_type():
    """The project with a line type of its own named Sinir (the recipe's SINIR, letters' case aside)."""
    s = zc.setup(work="road")
    s["settings"]["lineTypes"] = [copy.deepcopy(HERE_SINIR)]
    return s


def group_cases():
    """Docs/adr/4040: a recipe layer's groups, whole style and line type."""
    out = []
    path = fx("grup-stilli.json")
    went_a = dict(went("k-12", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı"), group=["Yol", "Toprak işleri"],
                  groupsAdded=["Toprak işleri"], lineTypeAdded="SINIR")
    said_a = {"code": "recipe_layer_created", "message": "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki "
              "stiliyle “Yol › Toprak işleri” grubuna eklendi. “Toprak işleri” grubu da eklendi. “SINIR” çizgi türü de "
              "tarifteki tanımıyla eklendi.", "path": "layers"}
    tree_a = tree_with(node("Toprak işleri", DEFAULT, "group", [node("Kazı sınırı", STYLE)]), into="grup")
    left_both = [{"drawing": "cz-1", "why": "exists", "message": EXISTS},
                 {"drawing": "cz-2", "why": "exists", "message": EXISTS}]
    out.append(case("Grubuyla ve bütün stiliyle eklenir: grup adıyla bulunur, alt grubu ve çizgi türü eklenir; ikinci kez "
                    "adıyla bulunur",
                    step({"path": path}, done({"produced": both(), "layers": [went_a], "revision": "$current"}, [said_a]),
                         expect=dict(STEPPED, layers=tree_a, settings={"lineTypes": [SINIR]})),
                    step({"path": path}, done({"produced": [], "left": left_both, "revision": "$current"}),
                         expect=SAME),
                    note="tarifte YOL › Toprak işleri: YOL katlanınca buradaki Yol; Toprak işleri yok, eklenir"))
    out.append(case("Plan grupları ve çizgi türünü söyler, yazmaz",
                    step({"path": path}, done({"produced": both(False), "layers": [went_a], "revision": "$current"},
                                              [said_a]), op="plan")))
    went_c = dict(went("k-13", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı"), group=["Toprak", "Sınırlar"],
                  groupsAdded=["Toprak", "Sınırlar"])
    said_c = {"code": "recipe_layer_created", "message": "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki "
              "stiliyle “Toprak › Sınırlar” grubuna eklendi. “Toprak”, “Sınırlar” grupları da eklendi.", "path": "layers"}
    tree_c = tree_with(node("Toprak", DEFAULT, "group", [node("Sınırlar", DEFAULT, "group", [
        node("Kazı sınırı", dict(STYLE, linePattern="Sinir"))])]), setup=own_line_type())
    out.append(case("Tarifin grupları yoksa eklenir; projede o adda çizgi türü varsa onunki kullanılır",
                    step({"path": fx("yeni-gruplu.json")},
                         done({"produced": both(), "layers": [went_c], "revision": "$current"}, [said_c]),
                         expect=dict(STEPPED, layers=tree_c, settings={"lineTypes": [HERE_SINIR]})),
                    setup="$cizgi", note="tarifin SINIR'ı burada Sinir adıyla başka desenle var: projeninki kalır"))
    plain = {k: v for k, v in STYLE.items() if k != "linePattern"}
    said_d = {"code": "recipe_layer_created", "message": "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki "
              "stiliyle eklendi. “YOKTUR” çizgi türü bu projede de tarifte de tanımlı değil; katman yerleşik türüyle "
              "çizilir.", "path": "layers"}
    out.append(case("Tarifte tanımı olmayan çizgi türü: katman yerleşik türüyle eklenir",
                    step({"path": fx("tanimsiz-cizgi-turlu.json")},
                         done({"produced": [produced("cz-1", "cz-1", "Düz", "Tümsek")],
                               "layers": [went("k-14", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı")],
                               "revision": "$current"}, [said_d]),
                         expect=dict(STEPPED, layers=tree_with(node("Kazı sınırı", plain))))))
    ova = "“Ova” kimlikli ya da adlı yüzey projede yok."
    by_name = [went("k-7", "sifir", "name", "Sıfır hattı")]
    out.append(case("Bütün çizimler: eklenen katmana çizilemeyen çizim bırakılır; katmanla eklenen gruplar ve çizgi türü "
                    "de geri alınır",
                    step({"path": fx("yeni-gruplu-eksik.json")},
                         done({"produced": both(), "left": [{"drawing": "cz-3", "why": "refused", "message": ova}],
                               "layers": by_name, "revision": "$current"}),
                         expect=dict(STEPPED, layers=zc.setup(work="road")["layers"], settings={"lineTypes": None}))))
    bad = "Tarifteki “cz-1” çiziminin katman tanımı (k-12) tutarsız: "
    for name, file_, why in [
        ("Stili tanımınkiyle tutmayan katman", "stili-tutmayan.json",
         "stilin rengi, çizgi türü ya da kalınlığı tanımınkiyle aynı değil."),
        ("Adı boş grup", "bos-gruplu.json", "grup adlarından biri boş."),
        ("Stilin adlamadığı çizgi türü tanımı", "adlanmayan-cizgi-turlu.json",
         "“KESIK” çizgi türünün tanımı var ama stil onu adlamıyor."),
        ("Proje çizgi türü olarak tutmayan tanım", "bosluklu-cizgi-turlu.json",
         "“ SINIR” çizgi tipinin adının başında ya da sonunda boşluk var."),
    ]:
        out.append(case(f"Tarifin katman tanımı tutarsız: {name.lower()}",
                        step({"path": fx(file_)}, failed("invalid_project_recipe", bad + why, "path"))))
    return out


# Docs/adr/4440, by hand: what a layer type holds (docs/adr/0622 §5), a zero line's object kind, and the words.
HOLDS = {"point": {"point"}, "line": {"line", "polyline", "arc", "spline", "nurbs", "circle", "ellipse"}}
LAYER_WORDS = {"point": "nokta katmanı", "line": "çizgi katmanı"}
TYPE_LABELS = {"point": "Nokta", "line": "Çizgi"}
BLEND_LABELS = {"multiply": "Çoğalt"}
ZERO_KIND, ZERO_KIND_WORDS = "polyline", "çoklu çizgi"


def transparency(opacity):
    """Saydamlık: 1 − the opacity in percent, its last bits rounded away (0.6 → “40”)."""
    p = round((1.0 - opacity) * 100.0, 9)
    return str(int(p)) if p == int(p) else repr(p)


def look_said(defn):
    """“Tarifteki gibi …”: what a layer added from `defn` takes beyond its style, in its panels' words."""
    parts = []
    c = defn.get("compose") or {}
    if c.get("opacity", 1.0) != 1.0:
        parts.append(f"saydamlığı %{transparency(c['opacity'])}")
    if c.get("blend", "normal") != "normal":
        parts.append(f"karışım kipi {BLEND_LABELS[c['blend']]}")
    if defn.get("geometry"):
        parts.append(f"türü {TYPE_LABELS[defn['geometry']]}")
    if defn.get("plot") is False:
        parts.append("yazdırılmaz")
    if not parts:
        return ""
    return " Tarifteki gibi " + (parts[0] if len(parts) == 1 else ", ".join(parts[:-1]) + " ve " + parts[-1]) + "."


def misfit_words(layer_name, geometry):
    """A zero line's polyline on a layer of `geometry` that does not hold it: the refusal's words."""
    assert ZERO_KIND not in HOLDS[geometry]
    return (f"“{layer_name}” bir {LAYER_WORDS[geometry]}; {ZERO_KIND_WORDS} bu katmana yazılmaz. Çizimin katmanını türüne "
            "uyan bir katman olarak layers ile verin ya da katmanın türünü Karışık yapın; hiçbir şey yazılmadı.")


def typed_point():
    """The project with the zero line's layer a point layer (it holds nothing yet)."""
    s = zc.setup(work="road")
    next(n for n in s["layers"] if n["id"] == "sifir")["geometry"] = "point"
    return s


def look_cases():
    """Docs/adr/4440: a recipe layer's composition, type and Yazdır."""
    out = []
    path = fx("saydam-katmanli.json")
    assert ZERO_KIND in HOLDS[K15["geometry"]]
    went_l = went("k-15", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı")
    said_l = {"code": "recipe_layer_created", "message": "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki "
              "stiliyle eklendi." + look_said(K15), "path": "layers"}
    added = dict(node("Kazı sınırı", PLAIN), compose=copy.deepcopy(K15["compose"]), geometry="line", plot=False)
    left_both = [{"drawing": "cz-1", "why": "exists", "message": EXISTS},
                 {"drawing": "cz-2", "why": "exists", "message": EXISTS}]
    out.append(case("Saydamlığı, karışım kipi, türü ve yazdırılmaması da tariften: eklenen katman onlarla; ikinci kez "
                    "adıyla bulunur",
                    step({"path": path}, done({"produced": both(), "layers": [went_l], "revision": "$current"},
                                              [said_l]),
                         expect=dict(STEPPED, layers=tree_with(added))),
                    step({"path": path}, done({"produced": [], "left": left_both, "revision": "$current"}),
                         expect=SAME),
                    note="tarifte %40 saydam (opaklık 0,6), Çoğalt, çizgi katmanı, yazdırılmaz; sıfır hattının çizgileri "
                         "çoklu çizgi: çizgi katmanına yazılır"))
    out.append(case("Plan eklenecek katmanın saydamlığını, türünü ve yazdırılmamasını söyler, yazmaz",
                    step({"path": path}, done({"produced": both(False), "layers": [went_l], "revision": "$current"},
                                              [said_l]), op="plan")))
    out.append(case("Adıyla bulunan katman kendi görünüşünü korur: tarifin saydamlığı ve türü uygulanmaz",
                    step({"path": fx("saydam-adli.json")},
                         done({"produced": both(), "layers": [went("k-7", "sifir", "name", "Sıfır hattı")],
                               "revision": "$current"}),
                         expect=dict(STEPPED, layers=zc.setup(work="road")["layers"]))))
    point = "Tarifin “cz-1” çizimi (Sıfır hattı): " + misfit_words("Kazı noktaları", "point")
    out.append(case("Tarifin nokta katmanına sıfır hattının çizgileri yazılmaz: ret, katman da eklenmez",
                    step({"path": fx("nokta-katmanli.json")}, failed("layer_geometry_mismatch", point, "layers"))))
    out.append(case("Plan da reddeder: nokta katmanına çizgi",
                    step({"path": fx("nokta-katmanli.json")}, failed("layer_geometry_mismatch", point, "layers"),
                         op="plan")))
    here = "Tarifin “cz-1” çizimi (Sıfır hattı): " + misfit_words("Sıfır hattı", "point")
    out.append(case("Bu projenin katmanı nokta katmanı: ret, plan da",
                    step({"path": fx("sifir-hatlari.json")}, failed("layer_geometry_mismatch", here, "layers"),
                         op="plan"),
                    step({"path": fx("sifir-hatlari.json")}, failed("layer_geometry_mismatch", here, "layers")),
                    setup="$nokta", note="eskiden plan geçiyor, yazım adımın sonunda bütünlük denetimine takılıyordu"))
    bad = "Tarifteki “cz-1” çiziminin katman tanımı (k-15) tutarsız: "
    for name, file_, why in [
        ("Katman çiziminde maske", "maskeli.json",
         "katman çiziminde maske var; maske projesinin öbür katmanlarını kimlikleriyle keser, tarifte tutulmaz."),
        ("Boş katman çizimi", "bos-cizimli.json", "katman çizimi boş: saydamlık da karışım kipi de yok."),
        ("Opaklığı 1'den büyük", "saydamligi-tutmayan.json", "katman çiziminde saydamlık 0 ile 1 arasında olmalı."),
    ]:
        out.append(case(f"Tarifin katman tanımı tutarsız: {name.lower()}",
                        step({"path": fx(file_)}, failed("invalid_project_recipe", bad + why, "path"))))
    return out


# Docs/adr/4860, by hand: a list's id made of its name (its letters folded to ASCII, “-” between words, a number after
# it when taken), a kind's name in the interface, and the words.
ASCII = str.maketrans({"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "I": "i", "İ": "i", "ö": "o", "Ö": "o",
                       "ş": "s", "Ş": "s", "ü": "u", "Ü": "u"})
KIND_LABELS = {"text": "Metin", "decimal": "Ondalık sayı"}


def list_id(name, taken):
    stem = ""
    for c in name.translate(ASCII):
        c = c.lower()
        if c.isascii() and c.isalnum():
            stem += c
        elif stem and not stem.endswith("-"):
            stem += "-"
    stem = stem.rstrip("-")[:56] or "liste"
    if stem not in taken:
        return stem
    return next(f"{stem}-{n}" for n in range(2, 10_000) if f"{stem}-{n}" not in taken)


def fields_here(domain):
    """The recipe's fields as the layer added here holds them: the kind linked to the list `domain` here."""
    out = copy.deepcopy(FIELDS)
    out[1]["domain"] = domain
    return out


def fields_said(lists):
    """What a layer added from K20 took of its fields: its three fields and subtypes, then each list as it went."""
    out = " Alanları (3) ve alt türleri tarifteki gibi."
    for name, went_, here in lists:
        out += {"linked": f" Alanları bu projenin aynı değerli “{name}” listesine bağlandı.",
                "added": f" “{name}” değer listesi de tarifteki tanımıyla eklendi.",
                "renamed": f" Bu projenin “{name}” listesi başka değerler taşıyor, değişmedi; tarifinki “{here}” adıyla "
                           "eklendi."}[went_]
    return out


def with_lists(*lists):
    """The road project with the value lists `lists`."""
    s = zc.setup(work="road")
    s["settings"]["domains"] = copy.deepcopy(list(lists))
    return s


def with_fields():
    """The road project with the zero line's layer holding a name and a depth written as text (no kind)."""
    s = zc.setup(work="road")
    next(n for n in s["layers"] if n["id"] == "sifir")["fields"] = [{"name": "Ad", "kind": "text"},
                                                                    {"name": "Derinlik", "kind": "text"}]
    return s


SAME_LIST = dict(SINIR_TURU, id="tur-listesi")
OTHER_LIST = dict(SINIR_TURU, values=SINIR_TURU["values"] + [{"code": "Y", "label": "Yarma"}])


def fields_cases():
    """Docs/adr/4860: a recipe layer's fields, the lists they link and its subtypes."""
    out = []
    path = fx("alanli-katmanli.json")
    created = "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki stiliyle eklendi."
    went_f = dict(went("k-20", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı"), domainsAdded=["Sınır türü"])
    said_f = {"code": "recipe_layer_created", "message": created + fields_said([("Sınır türü", "added", None)]),
              "path": "layers"}
    added = dict(node("Kazı sınırı", PLAIN), fields=fields_here("sinir-turu"), subtypes=copy.deepcopy(SUBTYPES))
    left_both = [{"drawing": "cz-1", "why": "exists", "message": EXISTS},
                 {"drawing": "cz-2", "why": "exists", "message": EXISTS}]
    out.append(case("Alanları, bağladıkları değer listesi ve alt türleriyle eklenir; ikinci kez adıyla bulunur",
                    step({"path": path}, done({"produced": both(), "layers": [went_f], "revision": "$current"},
                                              [said_f]),
                         expect=dict(STEPPED, layers=tree_with(added), settings={"domains": [SINIR_TURU]})),
                    step({"path": path}, done({"produced": [], "left": left_both, "revision": "$current"}),
                         expect=SAME),
                    note="bu projede değer listesi yok: tarifin Sınır türü listesi kendi kimliğiyle eklenir"))
    out.append(case("Plan eklenecek alanları ve listeyi söyler, yazmaz",
                    step({"path": path}, done({"produced": both(False), "layers": [went_f], "revision": "$current"},
                                              [said_f]), op="plan")))
    went_s = went("k-20", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı")
    said_s = dict(said_f, message=created + fields_said([("Sınır türü", "linked", None)]))
    added_s = dict(added, fields=fields_here("tur-listesi"))
    out.append(case("Bu projede aynı adlı ve aynı değerli liste varsa alanlar ona bağlanır, liste eklenmez",
                    step({"path": path}, done({"produced": both(), "layers": [went_s], "revision": "$current"},
                                              [said_s]),
                         expect=dict(STEPPED, layers=tree_with(added_s), settings={"domains": [SAME_LIST]})),
                    setup="$ayni-liste"))
    renamed = "Sınır türü_1"
    rid = list_id(renamed, {OTHER_LIST["id"]})
    went_r = dict(went_s, domainsAdded=[renamed])
    said_r = dict(said_f, message=created + fields_said([("Sınır türü", "renamed", renamed)]))
    added_r = dict(added, fields=fields_here(rid))
    out.append(case("Aynı adlı liste başka değerler taşıyorsa değişmez; tarifinki adının sonuna _1 alarak eklenir",
                    step({"path": path}, done({"produced": both(), "layers": [went_r], "revision": "$current"},
                                              [said_r]),
                         expect=dict(STEPPED, layers=tree_with(added_r, setup=with_lists(OTHER_LIST)),
                                     settings={"domains": [OTHER_LIST, dict(SINIR_TURU, id=rid, name=renamed)]})),
                    setup="$baska-liste", note="birleştirme yok: bu projenin listesinde Y (Yarma) da var"))
    differs = ("“Sıfır hattı” katmanı bu projede bulundu ve kendi alanlarını korur: tarifin “Tür” alanı onda yok; "
               f"“Derinlik” onda {KIND_LABELS['text']}, tarifte {KIND_LABELS['decimal']}. Alanları Katman özellikleri › "
               "Alanlar'da düzenleyin ya da çizimin katmanını layers ile verin.")
    out.append(case("Adıyla bulunan katman kendi alanlarını korur; tarifin onda olmayan ve başka türlü alanları söylenir",
                    step({"path": fx("alanli-adli.json")},
                         done({"produced": both(), "layers": [went("k-7", "sifir", "name", "Sıfır hattı")],
                               "revision": "$current"},
                              [{"code": "recipe_layer_fields", "message": differs, "path": "layers"}]),
                         expect=dict(STEPPED, layers=with_fields()["layers"], settings={"domains": None})),
                    setup="$alanli-sifir"))
    ova = "“Ova” kimlikli ya da adlı yüzey projede yok."
    out.append(case("Bütün çizimler: eklenen katmana çizilemeyen çizim bırakılır; katmanla eklenen liste de geri alınır",
                    step({"path": fx("alanli-eksik.json")},
                         done({"produced": both(), "left": [{"drawing": "cz-3", "why": "refused", "message": ova}],
                               "layers": [went("k-7", "sifir", "name", "Sıfır hattı")], "revision": "$current"}),
                         expect=dict(STEPPED, layers=zc.setup(work="road")["layers"], settings={"domains": None}))))
    bad = "Tarifteki “cz-1” çiziminin katman tanımı (k-20) tutarsız: "
    for name, file_, why in [
        ("Bağlı tablonun sütunu", "bagli-sutunlu.json",
         "“Kayıt günü” alanı bağlı tablonun sütununu taşıyor; bağ projesinindir, tarifte tutulmaz."),
        ("Bağlandığı liste tarifte yok", "listesiz-alanli.json",
         "“Tür” alanının bağlandığı “sinir-turu” değer listesi tarifte yok."),
        ("Alanın değerleri listesininki değil", "liste-tutmayan.json",
         "“Tür” alanının değerleri bağlandığı “Sınır türü” listesininkiyle aynı değil."),
        ("Alt türleri alanlara uymuyor", "alt-turu-tutmayan.json",
         "alt türlerinde: Katmanda “Yok” alanı yok; alt tür alanı katmanın bir alanıdır."),
        ("Aynı adlı iki alan", "iki-ad-alanli.json",
         "alanlarında: “Ad” kimliği iki alanda var; bir alanın kimliği ya da adı başka alanın kimliği olamaz."),
    ]:
        out.append(case(f"Tarifin katman tanımı tutarsız: {name.lower()}",
                        step({"path": fx(file_)}, failed("invalid_project_recipe", bad + why, "path"))))
    return out


def shaped_said(lists, forms=("Sınır formu",)):
    """What a layer added from K22 took (docs/adr/5240): its four fields and subtypes, its value combination, rule and
    index, its form as a draft, then each list as it went."""
    out = " Alanları (4) ve alt türleri tarifteki gibi. 1 değer bileşimi, 1 kayıt kuralı ve 1 dizin de tarifteki gibi."
    names = [f"“{n}”" for n in forms]
    out += f" {names[0]} formu taslak olarak eklendi; yayımlamak bu projenindir."
    return out + fields_said(lists)[len(" Alanları (3) ve alt türleri tarifteki gibi."):]


def shaped_here(domain):
    out = copy.deepcopy(SHAPED_FIELDS)
    out[1]["domain"] = domain
    return out


def with_form(form_id):
    """The road project with a form of the drainage layer under `form_id` (a note only)."""
    s = zc.setup(work="road")
    s["settings"]["forms"] = [{"id": form_id, "name": "Drenaj formu", "layer": "drenaj",
                               "draft": {"revision": 1, "items": [{"type": "note", "id": "not", "text": "Drenaj"}]}}]
    return s


def shaped_cases():
    """Docs/adr/5240: a recipe layer's value combinations, record rules, indexes and forms; `addFields`."""
    out = []
    path = fx("tanimli-katmanli.json")
    created = "Tarifin “Kazı sınırı” katmanı bu projede yoktu; tarifteki stiliyle eklendi."
    went_t = dict(went("k-20", ANY, "created", "Kazı sınırı", to_name="Kazı sınırı"), domainsAdded=["Sınır türü"],
                  formsAdded=["Sınır formu"])
    said_t = {"code": "recipe_layer_created", "message": created + shaped_said([("Sınır türü", "added", None)]),
              "path": "layers"}
    added = dict(node("Kazı sınırı", PLAIN), fields=shaped_here("sinir-turu"), subtypes=copy.deepcopy(SUBTYPES),
                 contingents=[copy.deepcopy(GROUP)], rules=[copy.deepcopy(RULE)], indexes=[copy.deepcopy(INDEX)])
    form_here = lambda id_: {"id": id_, "name": FORM["name"], "layer": ANY, "draft": copy.deepcopy(FORM["draft"])}  # noqa: E731
    out.append(case("Değer bileşimi, kayıt kuralı, dizin ve form da tarifteki gibi eklenir; form taslak olarak",
                    step({"path": path}, done({"produced": both(), "layers": [went_t], "revision": "$current"},
                                              [said_t]),
                         expect=dict(STEPPED, layers=tree_with(added),
                                     settings={"domains": [SINIR_TURU], "forms": [form_here("sinir-formu")]})),
                    note="yayım tarifin projesinin kararıdır; burada yeniden yayımlanır"))
    out.append(case("Plan eklenecek bileşimi, kuralı, dizini ve formu söyler, yazmaz",
                    step({"path": path}, done({"produced": both(False), "layers": [went_t], "revision": "$current"},
                                              [said_t]), op="plan")))
    drenaj = with_form("sinir-formu")["settings"]["forms"][0]
    out.append(case("Formun kimliği bu projede başka formunsa -2 ile eklenir",
                    step({"path": path}, done({"produced": both(), "layers": [went_t], "revision": "$current"},
                                              [said_t]),
                         expect=dict(STEPPED, layers=tree_with(added, setup=with_form("sinir-formu")),
                                     settings={"domains": [SINIR_TURU],
                                               "forms": [drenaj, form_here("sinir-formu-2")]})),
                    setup="$formlu"))
    ova = "“Ova” kimlikli ya da adlı yüzey projede yok."
    out.append(case("Çizimi gitmeyen eklenen katman formuyla ve listesiyle geri alınır",
                    step({"path": fx("tanimli-eksik.json")},
                         done({"produced": both(), "left": [{"drawing": "cz-3", "why": "refused", "message": ova}],
                               "layers": [went("k-7", "sifir", "name", "Sıfır hattı")], "revision": "$current"}),
                         expect=dict(STEPPED, layers=zc.setup(work="road")["layers"],
                                     settings={"domains": None, "forms": None}))))
    # addFields: the zero line's layer here holds a name and a depth written as text.
    kind_only = ("“Sıfır hattı” katmanı bu projede bulundu ve kendi alanlarını korur: “Derinlik” onda "
                 f"{KIND_LABELS['text']}, tarifte {KIND_LABELS['decimal']}. Alanları Katman özellikleri › Alanlar'da "
                 "düzenleyin ya da çizimin katmanını layers ile verin.")
    took = ("Tarifin “Tür” ve “Zemin” alanları “Sıfır hattı” katmanının alanlarının sonuna eklendi; katmanın kendi "
            "alanları ve öbür tanımları değişmedi.")
    here_after = with_fields()
    sifir = next(n for n in here_after["layers"] if n["id"] == "sifir")
    sifir["fields"] += [dict(copy.deepcopy(FIELDS[1]), domain="sinir-turu"), copy.deepcopy(ZEMIN)]
    went_a = dict(went("k-7", "sifir", "name", "Sıfır hattı"), domainsAdded=["Sınır türü"], fieldsAdded=["Tür", "Zemin"])
    out.append(case("Eksik alanları ekle: adıyla bulunan katmana tarifin onda olmayan alanları listesiyle eklenir",
                    step({"path": fx("tanimli-adli.json"), "addFields": True},
                         done({"produced": both(), "layers": [went_a], "revision": "$current"},
                              [{"code": "recipe_layer_fields_added",
                                "message": took + " “Sınır türü” değer listesi de tarifteki tanımıyla eklendi.",
                                "path": "layers"},
                               {"code": "recipe_layer_fields", "message": kind_only, "path": "layers"}]),
                         expect=dict(STEPPED, layers=here_after["layers"],
                                     settings={"domains": [SINIR_TURU], "forms": None})),
                    setup="$alanli-sifir",
                    note="kendi alanları, türü başka Derinlik de, değişmez; bileşim, kural, dizin ve form verilmez"))
    out.append(case("Eksik alanları ekle: planı yazmaz",
                    step({"path": fx("tanimli-adli.json"), "addFields": True},
                         done({"produced": both(False), "layers": [went_a], "revision": "$current"},
                              [{"code": "recipe_layer_fields_added",
                                "message": took + " “Sınır türü” değer listesi de tarifteki tanımıyla eklendi.",
                                "path": "layers"},
                               {"code": "recipe_layer_fields", "message": kind_only, "path": "layers"}]), op="plan"),
                    setup="$alanli-sifir"))
    rid = list_id("Sınır türü_1", {OTHER_LIST["id"]})
    here_r = with_fields()
    here_r["settings"]["domains"] = [copy.deepcopy(OTHER_LIST)]
    sifir_r = next(n for n in here_r["layers"] if n["id"] == "sifir")
    sifir_r["fields"] += [dict(copy.deepcopy(FIELDS[1]), domain=rid), copy.deepcopy(ZEMIN)]
    went_ar = dict(went_a, domainsAdded=["Sınır türü_1"])
    out.append(case("Eksik alanları ekle: aynı adlı başka değerli liste değişmez, tarifinki _1 ile",
                    step({"path": fx("tanimli-adli.json"), "addFields": True},
                         done({"produced": both(), "layers": [went_ar], "revision": "$current"},
                              [{"code": "recipe_layer_fields_added",
                                "message": took + " Bu projenin “Sınır türü” listesi başka değerler taşıyor, değişmedi; "
                                           "tarifinki “Sınır türü_1” adıyla eklendi.", "path": "layers"},
                               {"code": "recipe_layer_fields", "message": kind_only, "path": "layers"}]),
                         expect=dict(STEPPED, layers=here_r["layers"],
                                     settings={"domains": [OTHER_LIST, dict(SINIR_TURU, id=rid, name="Sınır türü_1")]})),
                    setup="$alanli-baska-liste"))
    differs = ("“Sıfır hattı” katmanı bu projede bulundu ve kendi alanlarını korur: tarifin “Tür”, “Zemin” alanları "
               f"onda yok; “Derinlik” onda {KIND_LABELS['text']}, tarifte {KIND_LABELS['decimal']}. Alanları Katman "
               "özellikleri › Alanlar'da düzenleyin ya da çizimin katmanını layers ile verin.")
    out.append(case("Eksik alanları eklemeden: bulunan katman değişmez, eksikleri söylenir",
                    step({"path": fx("tanimli-adli.json")},
                         done({"produced": both(), "layers": [went("k-7", "sifir", "name", "Sıfır hattı")],
                               "revision": "$current"},
                              [{"code": "recipe_layer_fields", "message": differs, "path": "layers"}]),
                         expect=dict(STEPPED, layers=with_fields()["layers"], settings={"domains": None})),
                    setup="$alanli-sifir"))
    out.append(case("Eksik alanları ekle: eklenen katmanda değişiklik yok",
                    step({"path": path, "addFields": True},
                         done({"produced": both(), "layers": [went_t], "revision": "$current"}, [said_t]),
                         expect=dict(STEPPED, layers=tree_with(added),
                                     settings={"domains": [SINIR_TURU], "forms": [form_here("sinir-formu")]}))))
    bad = "Tarifteki “cz-1” çiziminin katman tanımı (k-20) tutarsız: "
    for name, file_, why in [
        ("Bileşimin alanı katmanda yok", "bilesimi-tutmayan.json", "değer bileşimlerinde: Katmanda “Yok” alanı yok."),
        ("Aynı kural iki kez", "kurali-tutmayan.json", "kayıt kurallarında: “derinlik-arti” kuralı iki kez var."),
        ("Dizinin alanı yok", "dizini-tutmayan.json", ANY),
        ("Form katmanda olmayan alana bağlı", "formu-tutmayan.json",
         "“Sınır formu” formu katmanın alanlarında olmayan “Yok” alanına bağlı."),
        ("İki formun kimliği aynı", "iki-formlu.json", "iki formun kimliği aynı."),
    ]:
        out.append(case(f"Tarifin katman tanımı tutarsız: {name.lower()}",
                        step({"path": fx(file_)}, failed("invalid_project_recipe", why if why == ANY else bad + why,
                                                          "path"))))
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$mimari":
            c["setup"] = zc.setup(work="architecture")
        elif c.get("setup") == "$harita":
            c["setup"] = zc.setup(work="map")
        elif c.get("setup") == "$cift":
            c["setup"] = doubled()
        elif c.get("setup") == "$cizgi":
            c["setup"] = own_line_type()
        elif c.get("setup") == "$nokta":
            c["setup"] = typed_point()
        elif c.get("setup") == "$ayni-liste":
            c["setup"] = with_lists(SAME_LIST)
        elif c.get("setup") == "$baska-liste":
            c["setup"] = with_lists(OTHER_LIST)
        elif c.get("setup") == "$alanli-sifir":
            c["setup"] = with_fields()
        elif c.get("setup") == "$formlu":
            c["setup"] = with_form("sinir-formu")
        elif c.get("setup") == "$alanli-baska-liste":
            s_ = with_fields()
            s_["settings"]["domains"] = [copy.deepcopy(OTHER_LIST)]
            c["setup"] = s_
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.project.produce", "commandVersion": 1,
           "title": "Tariften üret",
           "note": "ADR 3240, 3640, 4040, 4440, 4860, 5240. Bağımsız başvuru scripts/fixtures/road_project_produce_cases.py: tarifler (İş grafiği'nin "
                   "graf biçimi) fixtures/recipes/v1 altında elle yazılır; sıfır hatlarının çizgi sayısı "
                   "road_terrain_zeroline_cases.py'den; KentOS kodu yok.",
           "setup": zc.setup(work="road"), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    files = {n: (json.dumps(v, ensure_ascii=False, indent=2) + "\n") for n, v in FILES.items()}
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text and all(
            (RECIPES / n).exists() and (RECIPES / n).read_text("utf-8") == t for n, t in files.items())
        print("Tariften üret ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    RECIPES.mkdir(parents=True, exist_ok=True)
    for n, t in files.items():
        (RECIPES / n).write_text(t, "utf-8")
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(found)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
