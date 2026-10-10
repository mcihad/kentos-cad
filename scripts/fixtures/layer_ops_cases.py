#!/usr/bin/env python3
"""Independent reference of the Katman tools (docs/adr/0217), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/layers.kcad (the drawing) and fixtures/processing/v1/layers.json (the tools run on it, in
the processing cases' format, fixtures/processing/README.md): Özniteliğe göre böl, Vektör katmanlarını birleştir,
Seçilenleri çıkar, Yinelenenleri sil and İfadeye göre sırala.

The rules (ADR 0217 §2–§7):

1. A tool's input leaves out the objects on locked layers (said as İşlemler's written input says it); a list of layers
   is read in the tree's order, layer by layer in the drawing's order.
2. A copy is the object with every field but its id, under its new layer.
3. Özniteliğe göre böl: an object's value is its fields' values trimmed, “(boş)” for none, joined by “ – ”; a layer per
   value in the order the values first come, in a new group named after the fields, each taking the look and fields of
   its value's first object's layer.
4. Vektör katmanlarını birleştir: copies, the layers in the tree's order; with a map only its targets (the column's
   cell when not blank, else the default when not blank); then the source layer field; into a layer with typed fields
   by their rules (layer_field_cases.py's `check`), the layer's defaults, a required field left empty: not added.
5. Seçilenleri çıkar: copies (or the objects moved) into the result layer; a new one takes the first object's layer's
   look and fields.
6. Yinelenenleri sil: the first of each key stays; a key is the fields' values trimmed (all attributes with a value
   when none are named) and, with Geometri de aynı olsun, the kind and the shape.
7. İfadeye göre sırala: values without a place (none, a number that is not finite) where Boşlar puts them; kinds
   truth value, number, text; false before true, numbers by value, texts in the natural order
   (feature_table_cases.py's `natural_cmp`); Azalan turns the values round; ties keep the input's order. Copies in
   that order, the order field 1, 2, 3 …
8. A new layer's id is `islem-` and its folded name's letters and digits, “-” for every other run; taken: “-2”, then
   “-2-3” (the processing runner's rule for a new layer).
"""

import json
import re
import sys
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from feature_table_cases import natural_cmp  # noqa: E402
from layer_field_cases import check, trim  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
DEFAULT_STYLE = {"lineType": "continuous", "lineWeight": 0.18}


def P(x, y):
    return {"x": E + x, "y": N + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def layer(i, name, color, fields=None, locked=False):
    node = {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    if fields:
        node["fields"] = fields
    return node


PARCEL_FIELDS = [{"name": "Ada", "kind": "integer"}, {"name": "Mahalle", "kind": "text"}]
STOCK_FIELDS = [
    {"name": "Ad", "kind": "text", "required": True},
    {"name": "Kullanım", "kind": "text"},
    {"name": "Kat", "kind": "integer"},
    {"name": "Durum", "kind": "text", "default": "Mevcut"},
]

LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("parsel", "Parsel", "#E5484D", PARCEL_FIELDS),
    layer("bina", "Bina", "#3E63DD"),
    layer("yol", "Yol", "#30A46C"),
    layer("kilitli", "Arşiv", "#8E8C99", locked=True),
    layer("hedef", "Yapı envanteri", "#F76B15", STOCK_FIELDS),
    layer("secilen", "Seçili işler", "#E54666"),
    layer("kullanim", "KULLANIM", "#FFC53D"),
    layer("roper", "Röper", "#00A2C7"),
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


ENTITIES = [
    # Parcels: 5 is 1 again (attributes and shape), 7 has 2's attributes elsewhere, 3's district has spaces, 4 has
    # none, 8 an empty one.
    ent(1, "polygon", "parsel", {"Ada": "101", "Parsel": "1", "Mahalle": "Merkez", "Kullanım": "Konut"}, pts=rect(0, 0, 20, 20)),
    ent(2, "polygon", "parsel", {"Ada": "101", "Parsel": "2", "Mahalle": "Yeni"}, pts=rect(20, 0, 40, 20)),
    ent(3, "polygon", "parsel", {"Ada": "101", "Parsel": "3", "Mahalle": " Merkez "}, pts=rect(40, 0, 60, 30)),
    ent(4, "polygon", "parsel", {"Ada": "102", "Parsel": "4"}, pts=rect(60, 0, 70, 10)),
    ent(5, "polygon", "parsel", {"Ada": "101", "Parsel": "1", "Mahalle": "Merkez", "Kullanım": "Konut"}, pts=rect(0, 0, 20, 20)),
    ent(6, "polygon", "parsel", {"Ada": "102", "Parsel": "6", "Mahalle": "Eski"}, pts=rect(70, 0, 90, 25)),
    ent(7, "polygon", "parsel", {"Ada": "101", "Parsel": "2", "Mahalle": "Yeni"}, pts=rect(20, 30, 40, 50)),
    ent(8, "polygon", "parsel", {"Ada": "103", "Parsel": "8", "Mahalle": ""}, pts=rect(90, 0, 100, 40)),
    # Buildings: 13 without a name, 14 with a storey count that is not a whole number.
    ent(11, "polygon", "bina", {"Kullanım": "Konut", "Kat": "3", "Ad": "A Blok"}, pts=rect(2, 2, 10, 10)),
    ent(12, "polygon", "bina", {"Kullanım": "Ticaret", "Kat": "1", "Ad": "Çarşı"}, pts=rect(22, 2, 30, 10)),
    ent(13, "polygon", "bina", {"Kullanım": "Konut", "Kat": "5"}, pts=rect(42, 2, 50, 10)),
    ent(14, "polygon", "bina", {"Kullanım": "Konut", "Kat": "iki", "Ad": "B Blok"}, pts=rect(62, 2, 68, 8)),
    ent(15, "polygon", "bina", {"Kullanım": "Ticaret", "Kat": "1", "Ad": "Pasaj"}, pts=rect(72, 2, 80, 10)),
    ent(21, "polyline", "yol", {"Ad": "Atatürk Cd.", "Tür": "Cadde"}, pts=[P(0, -5), P(100, -5)]),
    ent(22, "polyline", "yol", {"Ad": "Okul Sk.", "Tür": "Sokak"}, pts=[P(30, -5), P(30, -20)]),
    ent(31, "polygon", "kilitli", {"Ad": "Eski yapı"}, pts=rect(0, 60, 10, 70)),
    ent(32, "point", "kilitli", {"Ad": "Röper"}, p=P(5, 80)),
    # Bench marks in one place: 41 and 42 differ only in their elevation's zero's sign (one number), 43 is higher.
    ent(41, "point", "roper", {"Ad": "R1"}, p=P(5, 90), z=0.0),
    ent(42, "point", "roper", {"Ad": "R1"}, p=P(5, 90), z=-0.0),
    ent(43, "point", "roper", {"Ad": "R1"}, p=P(5, 90), z=0.5),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: katman işlemleri",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# The tools' new layers' looks (the layer parameters' `newLayerStyle` over the default style).
MERGED_STYLE = {"color": "#8E4EC6", **DEFAULT_STYLE}
EXTRACTED_STYLE = {"color": "#0090FF", **DEFAULT_STYLE}
ORDERED_STYLE = {"color": "#12A594", **DEFAULT_STYLE}


def node(lay):
    return next(n for n in LAYERS if n["id"] == lay)


def name_of(lay):
    return node(lay)["name"]


def objects(lay):
    return [e for e in ENTITIES if e["layerId"] == lay]


def locked(e):
    return node(e["layerId"])["locked"]


def fold(s):
    return trim(s).upper().replace("Ç", "C").replace("Ğ", "G").replace("İ", "I").replace("Ö", "O").replace("Ş", "S").replace("Ü", "U")


def free_name(name, siblings):
    """A node's name among these: the name, or with “ (2)”, “ (3)” … while a node there has it (folded)."""
    out, k = name, 2
    while any(fold(s) == fold(out) for s in siblings):
        out = f"{name} ({k})"
        k += 1
    return out


def new_id(name, taken):
    """The runner's id for a new layer: `islem-` and the folded name, “-2”, “-3” … appended while taken."""
    i = "islem-" + re.sub(r"[^a-z0-9]+", "-", fold(name).lower())
    k = 2
    while i in taken:
        i = f"{i}-{k}"
        k += 1
    taken.add(i)
    return i


def copy(e, lay, attrs=None):
    c = {k: v for k, v in e.items() if k != "id"}
    c["layerId"] = lay
    if attrs is not None:
        c["attrs"] = attrs
    return c


def plain_layer(i, name, style, fields=None, parent=None):
    out = {"id": i, "name": name, "style": style}
    if fields:
        out["fields"] = fields
    if parent is not None:
        out["parent"] = parent
    return out


def taken_ids():
    return {n["id"] for n in LAYERS}


def locked_note(label, n):
    return {"level": "warn", "text": f"“{label}”: {n} nesne kilitli katmanda olduğu için işleme alınmadı."}


def case(cid, title, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "layers.kcad", "run": {"tool": values.pop("_tool")}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def L(i):
    return {"scope": "layer", "layerId": i}


# ── Özniteliğe göre böl ─────────────────────────────────────────────────


def split(lay, fields, mode):
    inputs = [e for e in objects(lay) if not locked(e)]
    names = [trim(f) for f in fields.split(",") if trim(f)]
    groups = {}
    for e in inputs:
        parts = [trim(e["attrs"].get(f, "")) or "(boş)" for f in names]
        groups.setdefault(" – ".join(parts), []).append(e)
    taken = taken_ids()
    # The group right above the first object's layer, among the tree's top nodes here; its layers in it.
    group_name = free_name(", ".join(names), [n["name"] for n in LAYERS])
    group_id = new_id(group_name, taken)
    inside, layer_name, ids = [], {}, {}
    for v in groups:
        layer_name[v] = free_name(v, inside)
        inside.append(layer_name[v])
        ids[v] = new_id(layer_name[v], taken)
    layers = [plain_layer(ids[v], layer_name[v], node(es[0]["layerId"])["style"], node(es[0]["layerId"]).get("fields"), group_id) for v, es in groups.items()]
    value_of = {e["id"]: v for v, es in groups.items() for e in es}
    expect = {"status": "ok", "undo": "Özniteliğe göre böl", "layers": layers, "groups": [{"id": group_id, "name": group_name}]}
    if mode == "copy":
        expect["added"] = [copy(e, ids[value_of[e["id"]]]) for e in inputs]
    else:
        expect["updated"] = [{"id": e["id"], "attrs": e["attrs"], "layerId": ids[value_of[e["id"]]]} for e in inputs]
    done = "kopyalandı" if mode == "copy" else "taşındı"
    expect["summary"] = f"{len(inputs)} nesne {len(groups)} katmana bölündü ({done})."
    expect["outputs"] = {"count": len(groups), "table": {"columns": ["Değer", "Nesne"], "rows": [[v, str(len(es))] for v, es in groups.items()]}}
    return expect


# ── Vektör katmanlarını birleştir ───────────────────────────────────────


def merge(layer_ids, mapping, source_field, out):
    tree = [n["id"] for n in LAYERS]
    chosen = sorted(set(layer_ids), key=tree.index)
    log = []
    taken_layers = [lay for lay in chosen if not node(lay)["locked"]]
    left = sum(len(objects(lay)) for lay in chosen if node(lay)["locked"])
    if left:
        log.append(locked_note("Katmanlar", left))
    if "layerId" in out:
        out_id, out_name, out_new = out["layerId"], name_of(out["layerId"]), False
    else:
        out_id, out_name, out_new = new_id(out["newName"], taken_ids()), out["newName"], True
    fields = [] if out_new else node(out_id).get("fields", [])
    field = {f["name"]: f for f in fields}
    added, misfits, not_added = [], 0, 0
    for lay in taken_layers:
        for e in objects(lay):
            if mapping:
                attrs = {}
                for m in mapping:
                    t, s, d = trim(m["target"]), trim(m["source"]), m["default"]
                    cell = e["attrs"].get(s, "") if s else ""
                    value = cell if trim(cell) != "" else (d if trim(d) != "" else None)
                    if value is not None:
                        attrs[t] = value
            else:
                attrs = dict(e["attrs"])
            if trim(source_field):
                attrs[trim(source_field)] = name_of(lay)
            # Into typed fields: values by their rules, the defaults, the required fields.
            kept = {}
            for k, v in attrs.items():
                if k in field:
                    res = check(field[k], v)
                    if "error" in res:
                        misfits += 1
                        continue
                    v = res["value"]
                kept[k] = v
            for f in fields:
                if trim(kept.get(f["name"], "")) == "" and trim(f.get("default", "")) != "":
                    res = check(f, f["default"])
                    if "value" in res and res["value"] != "":
                        kept[f["name"]] = res["value"]
            if any(f.get("required") and trim(kept.get(f["name"], "")) == "" for f in fields):
                not_added += 1
                continue
            added.append(copy(e, out_id, kept))
    if misfits:
        log.append({"level": "warn", "text": f"{misfits} değer alanına uymadığı için yazılmadı."})
    if not_added:
        log.append({"level": "warn", "text": f"{not_added} nesne zorunlu alanı boş kaldığı için eklenmedi."})
    expect = {"status": "ok", "summary": f"{len(taken_layers)} katmandan {len(added)} nesne “{out_name}” katmanına birleştirildi.",
              "undo": "Vektör katmanlarını birleştir"}
    if log:
        expect["log"] = log
    if out_new:
        expect["layers"] = [plain_layer(out_id, out_name, MERGED_STYLE)]
    expect["added"] = added
    expect["outputs"] = {"count": len(added)}
    return expect


# ── Seçilenleri çıkar ───────────────────────────────────────────────────


def extract(selection, mode, out):
    chosen = [e for e in ENTITIES if e["id"] in selection]
    inputs = [e for e in chosen if not locked(e)]
    log = [locked_note("Nesneler", len(chosen) - len(inputs))] if len(inputs) < len(chosen) else []
    if "layerId" in out:
        out_id, out_name, out_new = out["layerId"], name_of(out["layerId"]), False
    else:
        out_id, out_name, out_new = new_id(out["newName"], taken_ids()), out["newName"], True
    expect = {"status": "ok", "undo": "Seçilenleri çıkar"}
    if log:
        expect["log"] = log
    if out_new:
        like = node(inputs[0]["layerId"])
        expect["layers"] = [plain_layer(out_id, out_name, like["style"], like.get("fields"))]
    if mode == "copy":
        expect["added"] = [copy(e, out_id) for e in inputs]
        n, done = len(inputs), "kopyalandı"
    else:
        moved = [e for e in inputs if e["layerId"] != out_id]
        expect["updated"] = [{"id": e["id"], "attrs": e["attrs"], "layerId": out_id} for e in moved]
        n, done = len(moved), "taşındı"
    expect["summary"] = f"{n} nesne “{out_name}” katmanına {done}."
    expect["outputs"] = {"count": n}
    return expect


# ── Yinelenenleri sil ───────────────────────────────────────────────────


def same_zero(v):
    """A zero's two signs are one number: −0 is written as 0 (as JavaScript's JSON writes it)."""
    if isinstance(v, float):
        return v + 0.0
    if isinstance(v, dict):
        return {k: same_zero(x) for k, x in v.items()}
    if isinstance(v, list):
        return [same_zero(x) for x in v]
    return v


def shape(e):
    kept = {k: v for k, v in e.items() if k not in ("id", "layerId", "attrs", "label", "color", "symbol", "lineWeight")}
    return json.dumps(same_zero(kept), sort_keys=True)


def dedupe(lay, fields, geometry):
    inputs = [e for e in objects(lay) if not locked(e)]
    names = [trim(f) for f in fields.split(",") if trim(f)]
    first, removed, groups = {}, [], {}
    for e in inputs:
        if names:
            values = tuple(trim(e["attrs"].get(f, "")) for f in names)
            shown = " – ".join(v or "(boş)" for v in values)
        else:
            values = tuple(sorted((k, trim(v)) for k, v in e["attrs"].items() if trim(v) != ""))
            shown = "; ".join(f"{k}: {v}" for k, v in values) or "—"
        key = (values, shape(e) if geometry else None)
        if key in first:
            removed.append(e["id"])
            groups.setdefault(key, [shown, 0])[1] += 1
        else:
            first[key] = e["id"]
    rows = [[shown, str(n)] for shown, n in groups.values()]
    expect = {"status": "ok", "removed": removed,
              "outputs": {"count": len(removed), "table": {"columns": ["Değer", "Silinen"], "rows": rows}}}
    if removed:
        expect["summary"] = f"{len(groups)} grupta {len(removed)} yinelenen nesne silindi."
        expect["undo"] = "Yinelenenleri sil"
    else:
        expect["summary"] = "Yinelenen nesne yok."
        expect["undo"] = None
    return expect


# ── İfadeye göre sırala ─────────────────────────────────────────────────


def area(e):
    pts = e["pts"]
    s = Fraction(0)
    for a, b in zip(pts, pts[1:] + pts[:1]):
        s += Fraction(a["x"]) * Fraction(b["y"]) - Fraction(b["x"]) * Fraction(a["y"])
    return abs(s) / 2


def evaluate(e, expression):
    if expression == "$alan":
        return float(area(e))
    return e["attrs"].get(expression)


def order(values, descending, nulls_first):
    def rank(v):
        return 0 if isinstance(v, bool) else 1 if isinstance(v, (int, float)) else 2

    def placed(v):
        return v is not None and not (isinstance(v, float) and (v != v or v in (float("inf"), float("-inf"))))

    def cmp_values(a, b):
        if rank(a) != rank(b):
            return -1 if rank(a) < rank(b) else 1
        if isinstance(a, str):
            return natural_cmp(a, b)
        return (a > b) - (a < b)

    def cmp(i, j):
        a, b = values[i], values[j]
        if placed(a) and placed(b):
            o = cmp_values(a, b)
            o = -o if descending else o
        elif placed(a):
            o = 1 if nulls_first else -1
        elif placed(b):
            o = -1 if nulls_first else 1
        else:
            o = 0
        return o or (i > j) - (i < j)

    return sorted(range(len(values)), key=cmp_to_key(cmp))


def order_by(lay, expression, descending, nulls_first, field, out_name):
    inputs = [e for e in objects(lay) if not locked(e)]
    values = [evaluate(e, expression) for e in inputs]
    at = order(values, descending, nulls_first)
    out_id = new_id(out_name, taken_ids())
    like = node(inputs[0]["layerId"])
    added = []
    for n, i in enumerate(at, start=1):
        attrs = dict(inputs[i]["attrs"])
        if trim(field):
            attrs[trim(field)] = str(n)
        added.append(copy(inputs[i], out_id, attrs))
    return {"status": "ok", "summary": f"{len(inputs)} nesne sıralanıp “{out_name}” katmanına yazıldı.", "undo": "İfadeye göre sırala",
            "layers": [plain_layer(out_id, out_name, like["style"], like.get("fields"))], "added": added,
            "outputs": {"count": len(inputs)}}


def build():
    cases = [
        case("split-copy", "Parselleri mahalleye göre böl: boşluklu ve boş değerler, yeni grup ve katmanlar parselin görünüşü ve alanlarıyla, kopyalar",
             {"_tool": "layers.splitByAttribute", "input": L("parsel"), "fields": "Mahalle", "mode": "copy"}, split("parsel", "Mahalle", "copy")),
        case("split-move", "Binaları kullanım ve kata göre böl: iki alan, nesneler taşınır",
             {"_tool": "layers.splitByAttribute", "input": L("bina"), "fields": "Kullanım, Kat", "mode": "move"}, split("bina", "Kullanım, Kat", "move")),
        case("split-name-taken", "Grubun adı ağaçta var: “ (2)” eklenir (büyük küçük harf ve Türkçe harfler katlanarak)",
             {"_tool": "layers.splitByAttribute", "input": L("bina"), "fields": "Kullanım", "mode": "copy"}, split("bina", "Kullanım", "copy")),
        case("split-locked", "Kilitli katmanı böl: çalıştırma başlamaz",
             {"_tool": "layers.splitByAttribute", "input": L("kilitli"), "fields": "Ad", "mode": "copy"},
             {"status": "invalid", "issues": [{"param": "input", "message": "“Nesneler”: bu katmandaki nesnelerin hepsi kilitli katmanda. Kilidi Katmanlar panelinden açın."}]}),
        case("merge-all", "Yol ve parseli birleştir: ağacın sırasıyla, bütün öznitelikler ve kaynak katman",
             {"_tool": "layers.merge", "layers": ["yol", "parsel"], "map": [], "sourceField": "Kaynak katman", "output": {"newName": "Birleşik"}},
             merge(["yol", "parsel"], [], "Kaynak katman", {"newName": "Birleşik"})),
        case("merge-typed", "Bina ve yolu tipli katmana eşleyerek birleştir: varsayılanlar, uymayan değer, zorunlu alanı boş nesne",
             {"_tool": "layers.merge", "layers": ["bina", "yol"],
              "map": [{"target": "Ad", "source": "Ad", "default": ""}, {"target": "Kullanım", "source": "Kullanım", "default": "Yol"},
                      {"target": "Kat", "source": "Kat", "default": ""}],
              "sourceField": "", "output": {"layerId": "hedef"}},
             merge(["bina", "yol"], [{"target": "Ad", "source": "Ad", "default": ""}, {"target": "Kullanım", "source": "Kullanım", "default": "Yol"},
                                     {"target": "Kat", "source": "Kat", "default": ""}], "", {"layerId": "hedef"})),
        case("merge-locked", "Kilitli katmanla birleştir: kilitli katmanın nesneleri alınmaz",
             {"_tool": "layers.merge", "layers": ["parsel", "kilitli"], "map": [], "sourceField": "Kaynak katman", "output": {"newName": "Birleşik"}},
             merge(["parsel", "kilitli"], [], "Kaynak katman", {"newName": "Birleşik"})),
        case("merge-one", "Tek katman: çalıştırma başlamaz",
             {"_tool": "layers.merge", "layers": ["parsel"], "map": [], "sourceField": "Kaynak katman", "output": {"newName": "Birleşik"}},
             {"status": "invalid", "issues": [{"param": "layers", "message": "“Katmanlar”: en az 2 katman seçin."}]}),
        case("extract-copy", "Seçilenleri yeni katmana kopyala: yeni katman ilk nesnenin katmanının görünüşü ve alanlarıyla",
             {"_tool": "layers.extract", "input": {"scope": "selection"}, "mode": "copy", "output": {"newName": "Seçilenler"}},
             extract([2, 7, 12], "copy", {"newName": "Seçilenler"}), selection=[2, 7, 12]),
        case("extract-move", "Seçilenleri var olan katmana taşı: kilitli katmandaki seçili nesne alınmaz",
             {"_tool": "layers.extract", "input": {"scope": "selection"}, "mode": "move", "output": {"layerId": "secilen"}},
             extract([21, 22, 31], "move", {"layerId": "secilen"}), selection=[21, 22, 31]),
        case("dedupe-exact", "Parsellerin yinelenenlerini sil: bütün öznitelikler ve geometri",
             {"_tool": "layers.deleteDuplicates", "input": L("parsel"), "fields": "", "geometry": True}, dedupe("parsel", "", True)),
        case("dedupe-fields", "Binaların kullanıma göre yinelenenlerini sil: geometrisiz",
             {"_tool": "layers.deleteDuplicates", "input": L("bina"), "fields": "Kullanım", "geometry": False}, dedupe("bina", "Kullanım", False)),
        case("dedupe-none", "Yolların yinelenenleri yok: çizim değişmez",
             {"_tool": "layers.deleteDuplicates", "input": L("yol"), "fields": "", "geometry": True}, dedupe("yol", "", True)),
        case("dedupe-zero", "Röper noktalarının yinelenenini sil: kotun sıfırının işareti tek sayı, yüksek olan kalır",
             {"_tool": "layers.deleteDuplicates", "input": L("roper"), "fields": "", "geometry": True}, dedupe("roper", "", True)),
        case("order-area", "Parselleri alana göre azalan sırala: eşitler girdinin sırasıyla, Sıra alanı",
             {"_tool": "layers.orderBy", "input": L("parsel"), "expression": "$alan", "direction": "desc", "nulls": "last",
              "orderField": "Sıra", "output": {"newName": "Sıralı"}},
             order_by("parsel", "$alan", True, False, "Sıra", "Sıralı")),
        case("order-text", "Parselleri mahalleye göre artan sırala: boşlar başta, doğal sıra, sıra alanı yok",
             {"_tool": "layers.orderBy", "input": L("parsel"), "expression": "Mahalle", "direction": "asc", "nulls": "first",
              "orderField": "", "output": {"newName": "Mahalleye göre"}},
             order_by("parsel", "Mahalle", False, True, "", "Mahalleye göre")),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-9, "documents": {}, "files": {}, "cases": cases}


def main():
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "layers.json", proc), (PROC_DIR / "layers.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("katman işlemlerinin durumları tutarlı: fixtures/processing/v1/layers.*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
