#!/usr/bin/env python3
"""The shared cases of Veri çıkarımı: `cad.extract.run`, `cad.extract.table` and Tabloyu güncelle of an extraction table
(`cad.tables.edit` refresh) (docs/adr/1030; plan/cad-advanced.md CAD-ADV-13).

    python3 scripts/fixtures/extract_command_cases.py           # writes the files
    python3 scripts/fixtures/extract_command_cases.py --check   # writes nothing; compares

Python's standard library and the independent KCAD v2 writer (scripts/fixtures/kcad_v2_reference.py), no KentOS code.
Every expected row, count, quantity, key, path, total and cell is worked out here from ADR 1030's rules:

- the drawings in the recipe's order (the open one first), each object in its order; an insert's block looked into with
  `nested` (an external reference's with `xrefs`, never one whose drawing the recipe reads itself), its pieces placed
  by every insert above them (exact rational arithmetic: the fixtures' geometry is dyadic, so the core's floating
  point is exact too); a dynamic block's visibility state hides its pieces; a piece on the block's layer is on its
  insert's; a hidden or frozen layer (or group) hides what is on it;
- each occurrence's path (`drawing#uid/local`, `/x:<source id>` in an external reference, `#dizi:<id>` for an array
  counted whole), counted once;
- a dimension's measured length from its points (aligned: their distance), its text as shown by the project's
  3 decimals, an override (a text without `<>`) words only;
- the join: the file's key column's rows by their trimmed key, the joined columns' words;
- grouping by the word columns (the joined ones aside), the quantities summed exactly, sorting (code points; numbers
  numerically, empty last), the total from the unrounded values, cells by the display rule (the exact binary value
  rounded at 7 decimals then at the column's, half away from zero);
- a refresh: hand edits kept while their source cell is as it was, the user's further columns following the keys, a
  conflict, a vanished edited row or an inserted row keeping the table.
"""
import argparse
import copy
import hashlib
import math
import json
import sys
from decimal import ROUND_HALF_UP, Decimal
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as kv  # noqa: E402  (the independent KCAD v2 writer)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
SRC = OUT / "extract"
BASE = "$fixture:commands/v1/extract"
SEP = "\x1f"
LINE = F(5, 3)  # docs/adr/1030 §7: a cell's further line, in text heights

# ── Display rule ────────────────────────────────────────────────────────────


def fixed(v, d):
    """`display::fixed`: the exact binary value rounded half away from zero at 7 decimals, then at `d`."""
    x = abs(Decimal(float(v)))
    if d >= 7:
        body = x.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    else:
        body = x.quantize(Decimal(1).scaleb(-7), rounding=ROUND_HALF_UP).quantize(
            Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(body, "f")
    if float(v) < 0 and any(c in "123456789" for c in text):
        text = "-" + text
    return text


# ── The drawings ───────────────────────────────────────────────────────────


def P(x, y):
    return {"x": float(x), "y": float(y)}


def st(color="ink"):
    return {"color": color, "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False, frozen=False):
    n = {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
         "style": st(), "children": []}
    if frozen:
        n["frozen"] = True
    return n


def obj(kind, i, layer_id, **f):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": {}, **f}


def line(i, layer_id, a, b):
    return obj("line", i, layer_id, a=P(*a), b=P(*b))


def rect(i, layer_id, x0, y0, w, h):
    return obj("polygon", i, layer_id, pts=[P(x0, y0), P(x0 + w, y0), P(x0 + w, y0 + h), P(x0, y0 + h)])


def insert(i, layer_id, block, at, scale=1.0, rotation=0.0, **f):
    return obj("insert", i, layer_id, block=block, p=P(*at), scale=float(scale), rotation=float(rotation), **f)


def uid(drawing, n):
    return f"0192f5a1-1030-7000-8{drawing}00-{n:012d}"


SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad", "drawingUnit": "m"}
QUARTER = 1.5707963267948966  # the core's exact quarter turn

SANDALYE = "0192f5a1-1030-7000-8000-00000000b001"
MASA = "0192f5a1-1030-7000-8000-00000000b002"
KAPI = "0192f5a1-1030-7000-8000-00000000b003"
XREF = "0192f5a1-1030-7000-8000-00000000b004"
X_SANDALYE = "0192f5a1-1030-7000-8000-00000000b005"
ARRAY_1 = "0192f5a1-1030-7000-8000-00000000a001"
ARRAY_2 = "0192f5a1-1030-7000-8000-00000000a002"


def attribute(tag, value, constant=False):
    a = {"tag": tag, "value": value, "p": P(0, 0), "height": 0.1, "rotation": 0}
    if constant:
        a["constant"] = True
    return a


def blocks():
    """Sandalye (a 0.5 square, KOD S-01), Masa (a 1.5 × 0.75 top and four Sandalye, KOD constant M-01) and Kapı (a
    dynamic block of two 0.875 leaves; its state Tek, the default, hides the second)."""
    return [
        {"id": SANDALYE, "name": "Sandalye", "base": P(0, 0), "units": "m",
         "entities": [rect(1, "", 0, 0, 0.5, 0.5)], "attributes": [attribute("KOD", "S-01")]},
        {"id": MASA, "name": "Masa", "base": P(0, 0), "units": "m",
         "entities": [rect(1, "", 0, 0, 1.5, 0.75),
                      insert(2, "", SANDALYE, (0, -0.75)), insert(3, "", SANDALYE, (1, -0.75)),
                      insert(4, "", SANDALYE, (0, 1)), insert(5, "", SANDALYE, (1, 1))],
         "attributes": [attribute("KOD", "M-01", constant=True)]},
        {"id": KAPI, "name": "Kapı", "base": P(0, 0), "units": "m",
         "entities": [line(1, "", (0, 0), (0.875, 0)), line(2, "", (0.875, 0), (1.75, 0))],
         "dynamic": {"parameters": [{"kind": "visibility", "name": "Tip",
                                     "states": [{"name": "Tek", "hidden": [2]}, {"name": "Çift"}]}]}},
    ]


def kat2():
    """The second drawing (kat2.kcad): a Masa, two Sandalye, a Kapı opened Çift and a three-chair array."""
    ents = [insert(1, "mobilya", MASA, (0, 0)),
            insert(2, "mobilya", SANDALYE, (4, 0)),
            dict(insert(3, "mobilya", SANDALYE, (4, 1)), attrs={"KOD": "S-02"}),
            insert(4, "kapi", KAPI, (8, 0), parameters={"Tip": "Çift"}),
            insert(5, "mobilya", SANDALYE, (0, 4)), insert(6, "mobilya", SANDALYE, (1, 4)),
            insert(7, "mobilya", SANDALYE, (2, 4))]
    uids = [uid(2, k) for k in range(1, len(ents) + 1)]
    return {
        "format": "kentos.document", "version": 2, "name": "Kat 2", "uids": uids, "settings": SETTINGS,
        "origin": P(0, 0),
        "layers": [layer("a", "Genel"), layer("mobilya", "Mobilya"), layer("kapi", "Kapı")],
        "activeLayer": "a", "styles": {"items": [], "categories": []}, "blocks": blocks(), "entities": ents,
        "arrays": [{"id": ARRAY_2, "block": SANDALYE, "layerId": "mobilya", "p": P(0, 4), "scale": 1.0,
                    "rotation": 0.0, "layout": {"kind": "grid", "rows": 1, "cols": 3, "dx": 1.0, "dy": 0.0},
                    "items": [{"uid": uids[4]}, {"uid": uids[5]}, {"uid": uids[6]}]}],
    }


SIRA = "0192f5a1-1030-7000-8000-00000000b006"


def kat3():
    """A drawing of a dynamic Sıra (row) block (docs/adr/1131 §2): its frame and a Sandalye, an array action on its
    linear Boy (0 → 1 m, 0.5 m apart) copying the Sandalye, a visibility state Boş hiding it. Two Sıra at Boy 2 m (three
    Sandalye each): one Dolu, one Boş (its three hidden, the array's two copies too); a third Sıra at the default Boy on
    a hidden layer."""
    sira = {"id": SIRA, "name": "Sıra", "base": P(0, 0), "units": "m",
            "entities": [line(1, "", (0, 0), (1, 0)), insert(2, "", SANDALYE, (0, 0.5))],
            "dynamic": {"parameters": [{"kind": "linear", "name": "Boy", "from": P(0, 0), "to": P(1, 0)},
                                       {"kind": "visibility", "name": "Görünüm",
                                        "states": [{"name": "Dolu"}, {"name": "Boş", "hidden": [2]}]}],
                        "actions": [{"kind": "array", "parameter": "Boy", "pieces": [2], "spacing": 0.5}]}}
    ents = [insert(1, "mobilya", SIRA, (0, 0), parameters={"Boy": 2.0}),
            insert(2, "mobilya", SIRA, (0, 3), parameters={"Boy": 2.0, "Görünüm": "Boş"}),
            insert(3, "gizli", SIRA, (0, 6))]
    return {
        "format": "kentos.document", "version": 2, "name": "Kat 3", "uids": [uid(3, k) for k in range(1, 4)],
        "settings": SETTINGS, "origin": P(0, 0),
        "layers": [layer("a", "Genel"), layer("mobilya", "Mobilya"), layer("gizli", "Gizli", visible=False)],
        "activeLayer": "a", "styles": {"items": [], "categories": []}, "blocks": blocks() + [sira],
        "entities": ents,
    }


KAT3 = kat3()

KAT2 = kat2()
KAT2_BYTES = kv.container(kv.document(KAT2))
KAT2_DIGEST = hashlib.sha256(KAT2_BYTES).hexdigest()

CSV = "Blok;Açıklama;Birim fiyat\nMasa;Çalışma masası 150x75;2500\nSandalye;Ofis sandalyesi;750\nDolap;Dosya dolabı;1800\n"
CSV_REPEATED = "Blok;Açıklama\nMasa;Bir\nMasa;İki\n"
CSV_DIGEST = hashlib.sha256(CSV.encode()).hexdigest()


def kat1():
    """The open drawing: two Masa (the second turned a quarter and twice as large), three Sandalye (one KOD S-02), one
    on a hidden layer, a 2 × 3 Sandalye array with one item removed, two Kapı (one opened Çift and twice as large), three
    wall lines 43/128 m long, two dimensions (one whose text overrides its 4 m measure with “5.00”) and kat2.kcad as
    an external reference holding two lines."""
    ents = [
        insert(1, "mobilya", MASA, (0, 0), attrs={"KOD": "X"}),
        insert(2, "mobilya", MASA, (8, 0), scale=2, rotation=QUARTER),
        insert(3, "mobilya", SANDALYE, (0, 4)),
        insert(4, "mobilya", SANDALYE, (1, 4)),
        dict(insert(5, "mobilya", SANDALYE, (2, 4)), attrs={"KOD": "S-02"}),
        insert(6, "gizli", SANDALYE, (3, 4)),
        insert(7, "mobilya", SANDALYE, (0, 8)), insert(8, "mobilya", SANDALYE, (1, 8)),
        insert(9, "mobilya", SANDALYE, (2, 8)), insert(10, "mobilya", SANDALYE, (0, 9)),
        insert(11, "mobilya", SANDALYE, (2, 9)),
        insert(12, "kapi", KAPI, (12, 0)),
        insert(13, "kapi", KAPI, (12, 4), scale=2, parameters={"Tip": "Çift"}),
        line(14, "duvar", (0, -4), (0.3359375, -4)),
        line(15, "duvar", (1, -4), (1.3359375, -4)),
        line(16, "duvar", (2, -4), (2.3359375, -4)),
        obj("dimension", 17, "olcu", a=P(0, -6), b=P(4, -6), offset=1.0, height=0.25, text="5.00"),
        obj("dimension", 18, "olcu", a=P(0, -8), b=P(0, -5), offset=1.0, height=0.25),
        insert(19, "ref", XREF, (20, 0)),
    ]
    uids = [uid(1, k) for k in range(1, len(ents) + 1)]
    xref = {"id": XREF, "name": "Kat2", "base": P(0, 0), "units": "m",
            "entities": [line(1, "", (0, 0), (2, 0)), line(2, "", (0, 0), (0, 1.5))],
            "xref": {"path": f"{BASE}/kat2.kcad", "relative": "kat2.kcad", "digest": KAT2_DIGEST,
                     "objects": [uid(2, 91), uid(2, 92)]}}
    return {
        "format": "kentos.document", "version": 2, "name": "Kat 1", "uids": uids, "settings": SETTINGS,
        "origin": P(0, 0),
        "layers": [layer("a", "Genel"), layer("mobilya", "Mobilya"), layer("gizli", "Gizli", visible=False),
                   layer("kapi", "Kapı"), layer("duvar", "Duvar"), layer("olcu", "Ölçü"), layer("ref", "Referans"),
                   layer("kilitli", "Kilitli", locked=True),
                   dict(layer("grup", "Grup"), type="group", children=[layer("alt", "Alt")])],
        "activeLayer": "a", "styles": {"items": [], "categories": []},
        "blocks": blocks() + [xref], "entities": ents,
        "arrays": [{"id": ARRAY_1, "block": SANDALYE, "layerId": "mobilya", "p": P(0, 8), "scale": 1.0,
                    "rotation": 0.0, "layout": {"kind": "grid", "rows": 2, "cols": 3, "dx": 1.0, "dy": 1.0},
                    "items": [{"uid": uids[6]}, {"uid": uids[7]}, {"uid": uids[8]}, {"uid": uids[9]}, {},
                              {"uid": uids[10]}]}],
    }


KAT1 = kat1()

# ── Placement (exact) ──────────────────────────────────────────────────────


def placement(p, s, rot, base):
    """An insert's similarity as an exact affine [a, b, c, d, e, f]: x' = a·x + c·y + e, y' = b·x + d·y + f."""
    turns = {0.0: (0, 1), QUARTER: (1, 0)}
    sn, cs = turns[rot]
    s = F(s)
    a, b, c, d = s * cs, s * sn, -s * sn, s * cs
    bx, by = F(base["x"]), F(base["y"])
    return [a, b, c, d, F(p["x"]) - (a * bx + c * by), F(p["y"]) - (b * bx + d * by)]


def compose(m2, m1):
    a2, b2, c2, d2, e2, f2 = m2
    a1, b1, c1, d1, e1, f1 = m1
    return [a2 * a1 + c2 * b1, b2 * a1 + d2 * b1, a2 * c1 + c2 * d1, b2 * c1 + d2 * d1,
            a2 * e1 + c2 * f1 + e2, b2 * e1 + d2 * f1 + f2]


IDENTITY = [F(1), F(0), F(0), F(1), F(0), F(0)]


def apply(m, p):
    x, y = F(p["x"]), F(p["y"])
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


def length_of(e, m):
    """A line's length (axis aligned in the fixtures: exact)."""
    if e["kind"] != "line":
        return None
    (ax, ay), (bx, by) = apply(m, e["a"]), apply(m, e["b"])
    assert ax == bx or ay == by, "the fixtures' lines run along an axis"
    return abs(bx - ax) + abs(by - ay)


def area_of(e, m):
    if e["kind"] != "polygon":
        return None
    pts = [apply(m, p) for p in e["pts"]]
    s = sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))
    return abs(s) / 2


def scale_of(m):
    """A similarity's length factor (its first column's length; exact for the fixtures' quarter turns)."""
    a, b = m[0], m[1]
    return abs(a) + abs(b)


# ── The reference extraction ──────────────────────────────────────────────

KIND = {"insert": "Blok", "line": "Çizgi", "polyline": "Çoklu çizgi", "polygon": "Alan", "circle": "Daire",
        "arc": "Yay", "dimension": "Ölçü", "text": "Yazı", "point": "Nokta", "array": "Dizi"}


def quantity(c):
    return c["value"]["of"] in ("count", "length", "area", "measured", "computed")


def heading(c):
    v = c["value"]
    of = v["of"]
    if of == "count":
        unit = "adet"
    elif of in ("length", "measured"):
        unit = v.get("unit", "m")
    elif of == "area":
        unit = {"m2": "m²", "donum": "dönüm", "ha": "ha"}[v.get("unit", "m2")]
    elif of == "computed":
        unit = v.get("unit", "")
    else:
        unit = ""
    # The unit is not said twice: Adet, not “Adet (adet)”.
    return f"{c['label']} ({unit})" if unit and c["label"].strip().lower() != unit else c["label"]


def decimals(recipe, j):
    c = recipe["columns"][j]
    if "decimals" in c:
        return c["decimals"]
    of = c["value"]["of"]
    return {"count": 0, "length": SETTINGS["lengthDecimals"], "measured": SETTINGS["lengthDecimals"],
            "area": SETTINGS["areaDecimals"]}.get(of) if of != "computed" else decimals(recipe, c["value"]["column"])


def layer_info(doc):
    out = {}

    def walk(nodes, hidden):
        for n in nodes:
            h = hidden or not n["visible"] or n.get("frozen", False)
            out[n["id"]] = (n["name"], h)
            walk(n["children"], h)

    walk(doc["layers"], False)
    return out


def dimension_shown(e, measured):
    text = e.get("text")
    written = fixed(measured, SETTINGS["lengthDecimals"])
    if text:
        return text.replace("<>", written)
    return written


def varied(dynamic, pieces, values):
    """A dynamic block's pieces as an array action copies them (ADR 0319 §4): on a linear parameter from `from` to
    `to` (its default the distance), `n = ⌊now/spacing + 1e-9⌋ − ⌊default/spacing + 1e-9⌋ + 1` (1 to 1000) items, the
    chosen pieces copied `k · spacing` along the line for k = 1 … n − 1, each copy a new local id after the largest.
    Each piece: (the piece as placed, its index in the definition, its local id)."""
    params = {p["name"]: p for p in dynamic["parameters"]}
    out = list(pieces)
    next_id = max([q[2] for q in out], default=0) + 1
    for a in dynamic.get("actions", []):
        p = params.get(a["parameter"])
        if a["kind"] != "array" or p is None or p["kind"] != "linear":
            continue
        dx, dy = p["to"]["x"] - p["from"]["x"], p["to"]["y"] - p["from"]["y"]
        length = math.hypot(dx, dy)
        now = values.get(p["name"], length)
        held = lambda v: math.floor(v / a["spacing"] + 1e-9)
        n = int(min(max(held(now) - held(length) + 1, 1), 1000))
        chosen = [q for q in out if q[2] in a["pieces"]]
        for k in range(1, n):
            sx, sy = dx / length * a["spacing"] * k, dy / length * a["spacing"] * k
            for piece, src, _ in chosen:
                copy = dict(piece, id=next_id, p={"x": piece["p"]["x"] + sx, "y": piece["p"]["y"] + sy})
                out.append((copy, src, next_id))
                next_id += 1
    return out


def extract(recipe, docs, join_rows=None):
    """`docs`: (path, name, document) in the recipe's order; the open drawing's path is empty."""
    cols = recipe["columns"]
    kinds = set(recipe.get("kinds") or ["insert"])
    blocks_named = {b.lower() for b in recipe.get("blocks", [])}
    layers_named = set(recipe.get("layers", []))
    scope = recipe.get("scope", {})
    nested, xrefs, hidden_too = scope.get("nested", False), scope.get("xrefs", False), scope.get("hidden", False)
    whole = scope.get("arrays", "items") == "whole"
    notes = dict(hidden=0, hiddenStates=0, unloaded=[], readApart=[], repeated=0, angles=0, overrides=0,
                 unmatched=[], unused=[], tooDeep=0)
    occ = []
    seen = set()
    read_paths = {p for p, _, _ in docs if p}

    def read(e, defn, layer_name, path, m, drawing_name):
        words, values = [], []
        for c in cols:
            v = c["value"]
            of = v["of"]
            w, q = "", None
            if of == "drawing":
                w = drawing_name
            elif of == "kind":
                w = KIND[e["kind"]]
            elif of == "block":
                w = defn["name"] if defn else ""
            elif of == "layer":
                w = layer_name
            elif of == "variant":
                if defn and defn.get("dynamic") and e["kind"] == "insert":
                    w = "; ".join(f"{k}={val}" for k, val in sorted(e.get("parameters", {}).items()))
            elif of == "attribute":
                tag = v["tag"]
                a = next((x for x in (defn or {}).get("attributes", []) if x["tag"] == tag), None)
                if a is not None and e["kind"] == "insert":
                    w = a.get("value", "") if a.get("constant") else e["attrs"].get(tag, a.get("value", ""))
                else:
                    w = e["attrs"].get(tag, "")
            elif of == "count":
                q = F(1)
            elif of == "length":
                q = length_of(e, m)
                q = None if q is None else q * {"m": 1, "cm": 100, "mm": 1000, "km": F(1, 1000)}[v.get("unit", "m")]
            elif of == "area":
                q = area_of(e, m)
                q = None if q is None else q / {"m2": 1, "donum": 1000, "ha": 10000}[v.get("unit", "m2")]
            elif of == "measured":
                if e["kind"] == "dimension":
                    q = (abs(F(e["b"]["x"]) - F(e["a"]["x"])) + abs(F(e["b"]["y"]) - F(e["a"]["y"]))) * scale_of(m)
            elif of == "shown":
                if e["kind"] == "dimension":
                    meas = (abs(F(e["b"]["x"]) - F(e["a"]["x"])) + abs(F(e["b"]["y"]) - F(e["a"]["y"]))) * scale_of(m)
                    w = dimension_shown(e, meas)
            elif of == "overridden":
                if e["kind"] == "dimension":
                    w = "evet" if (e.get("text") and "<>" not in e["text"]) else "hayır"
            elif of == "computed":
                src = values[v["column"]]
                q = None if src is None else F(float(src) * float(v["factor"]))
            words.append(w)
            values.append(q)
        return dict(path=path, words=words, values=values)

    for dpath, dname, doc in docs:
        layers = layer_info(doc)
        defs = {b["id"]: b for b in doc.get("blocks", [])}
        item_of = {}
        if whole:
            for k, a in enumerate(doc.get("arrays", [])):
                for it in a["items"]:
                    if "uid" in it:
                        item_of[it["uid"]] = k
        placed = set()

        def hide(state):
            notes["hidden"] += 1
            if state:
                notes["hiddenStates"] += 1

        def visit(e, path, m, parent_layer, hidden, depth, state=False):
            lid = e["layerId"] or (parent_layer if parent_layer is not None else "")
            lname, lh = layers.get(lid, (lid, False))
            hid = hidden or lh
            takes_layer = not layers_named or lname in layers_named
            if e["kind"] == "insert":
                defn = defs.get(e["block"])
                x = defn.get("xref") if defn else None
                into_x = x is not None and xrefs
                takes = (not into_x and "insert" in kinds and takes_layer
                         and (not blocks_named or defn["name"].lower() in blocks_named))
                if takes:
                    if hid and not hidden_too:
                        hide(state)
                    elif path in seen:
                        notes["repeated"] += 1
                    else:
                        seen.add(path)
                        occ.append(read(e, defn, lname, path, m, dname))
                if defn is None:
                    return
                if x is not None:
                    if not xrefs:
                        return
                    if x.get("unloaded"):
                        if defn["name"] not in notes["unloaded"]:
                            notes["unloaded"].append(defn["name"])
                        return
                    if x.get("relative") in read_paths:
                        if defn["name"] not in notes["readApart"]:
                            notes["readApart"].append(defn["name"])
                        return
                elif not nested:
                    return
                inner = compose(m, placement(e["p"], e["scale"], e["rotation"], defn["base"]))
                hidden_ids = set()
                pieces = [(piece, k, piece["id"]) for k, piece in enumerate(defn["entities"])]
                if defn.get("dynamic"):
                    values = e.get("parameters", {})
                    params = {p["name"]: p for p in defn["dynamic"]["parameters"]}
                    pieces = varied(defn["dynamic"], pieces, values)
                    for p in defn["dynamic"]["parameters"]:
                        if p["kind"] == "visibility":
                            st_ = values.get(p["name"], p["states"][0]["name"])
                            hidden_ids |= set(next(s for s in p["states"] if s["name"] == st_).get("hidden", []))
                # Drawn pieces first, then those the visibility state hides, by the id of the piece each is or was
                # copied from (an array action's copies of a hidden piece are hidden with it, docs/adr/1131 §2).
                order = ([q for q in pieces if defn["entities"][q[1]]["id"] not in hidden_ids]
                         + [q for q in pieces if defn["entities"][q[1]]["id"] in hidden_ids])
                for piece, k, local in order:
                    seg = f"x:{x['objects'][k]}" if x is not None else str(local)
                    by_state = defn["entities"][k]["id"] in hidden_ids
                    visit(piece, f"{path}/{seg}", inner, lid, hid or by_state, depth + 1, state or by_state)
                return
            if e["kind"] not in kinds or not takes_layer:
                return
            if hid and not hidden_too:
                hide(state)
                return
            if path in seen:
                notes["repeated"] += 1
                return
            seen.add(path)
            if e["kind"] == "dimension" and e.get("text") and "<>" not in e["text"]:
                notes["overrides"] += 1
            occ.append(read(e, None, lname, path, m, dname))

        for u, e in zip(doc["uids"], doc["entities"]):
            if u in item_of:
                k = item_of[u]
                if k not in placed:
                    placed.add(k)
                    a = doc["arrays"][k]
                    defn = defs[a["block"]]
                    lname, lh = layers[a["layerId"]]
                    takes = ("insert" in kinds and (not layers_named or lname in layers_named)
                             and (not blocks_named or defn["name"].lower() in blocks_named))
                    if takes:
                        if lh and not hidden_too:
                            notes["hidden"] += 1
                        else:
                            words, values = [], []
                            for c in cols:
                                of = c["value"]["of"]
                                words.append({"drawing": dname, "kind": KIND["array"], "block": defn["name"],
                                              "layer": lname}.get(of, ""))
                                q = F(1) if of == "count" else None
                                if of == "computed":
                                    src = values[c["value"]["column"]]
                                    q = None if src is None else F(float(src) * float(c["value"]["factor"]))
                                values.append(q)
                            seen.add(f"{dpath}#dizi:{a['id']}")
                            occ.append(dict(path=f"{dpath}#dizi:{a['id']}", words=words, values=values))
                continue
            visit(e, f"{dpath}#{u}", IDENTITY, None, False, 0)

    quant = [quantity(c) for c in cols]
    joined = [c["value"]["of"] == "joined" for c in cols]
    if recipe.get("join"):
        j = recipe["join"]
        head, *rest = join_rows
        key = [h.strip() for h in head].index(j["key"])
        by = {}
        order = []
        for r in rest:
            by[r[key].strip()] = r
            order.append(r[key].strip())
        used = set()
        for o in occ:
            k = o["words"][j["on"]].strip()
            if k in by:
                used.add(k)
                for c, col in enumerate(cols):
                    if joined[c]:
                        o["words"][c] = by[k][[h.strip() for h in head].index(col["value"]["field"])]
            elif k not in notes["unmatched"]:
                notes["unmatched"].append(k)
        notes["unused"] = [k for k in order if k not in used]

    def cell(c, v, w):
        if quant[c]:
            return "" if v is None else fixed(v, decimals(recipe, c))
        return w

    rows = []
    if recipe.get("group"):
        at = {}
        for o in occ:
            k = SEP.join(o["words"][c] for c in range(len(cols)) if not quant[c] and not joined[c])
            if k not in at:
                at[k] = len(rows)
                rows.append(dict(key=k, words=list(o["words"]), sums=[None] * len(cols), members=[]))
            r = rows[at[k]]
            r["members"].append(o["path"])
            for c, v in enumerate(o["values"]):
                if quant[c] and v is not None:
                    r["sums"][c] = v if r["sums"][c] is None else r["sums"][c] + v
        rows = [dict(key=r["key"], cells=[cell(c, r["sums"][c], r["words"][c]) for c in range(len(cols))],
                     values=[r["sums"][c] if quant[c] else None for c in range(len(cols))], members=r["members"])
                for r in rows]
    else:
        rows = [dict(key=o["path"], cells=[cell(c, o["values"][c], o["words"][c]) for c in range(len(cols))],
                     values=[o["values"][c] if quant[c] else None for c in range(len(cols))], members=[o["path"]])
                for o in occ]
    for s in reversed(recipe.get("sort", [])):
        c, down = s["column"], s.get("descending", False)
        if quant[c]:
            present = [r for r in rows if r["values"][c] is not None]
            empty = [r for r in rows if r["values"][c] is None]
            rows = sorted(present, key=lambda r: r["values"][c], reverse=down) + empty
        else:
            rows = sorted(rows, key=lambda r: r["cells"][c], reverse=down)
    totals = None
    if recipe.get("total"):
        totals = []
        for c in range(len(cols)):
            if quant[c]:
                totals.append(sum((o["values"][c] for o in occ if o["values"][c] is not None), F(0)))
            else:
                totals.append(None)
    out = dict(headings=[heading(c) for c in cols], quantities=quant,
               rows=[dict(key=r["key"], cells=r["cells"],
                          values=[None if v is None else float(v) for v in r["values"]],
                          members=r["members"]) for r in rows],
               occurrences=len(occ), notes=notes)
    if totals is not None:
        label = next(c for c in range(len(cols)) if not quant[c])
        out["total"] = [fixed(t, decimals(recipe, c)) if t is not None else ("Toplam" if c == label else "")
                        for c, t in enumerate(totals)]
        out["totals"] = [None if t is None else float(t) for t in totals]
    return out


def cells_of(x):
    """The table's cells of a reference result: headings, rows, total."""
    return [x["headings"]] + [r["cells"] for r in x["rows"]] + ([x["total"]] if "total" in x else [])


# ── The recipes ───────────────────────────────────────────────────────────


def col(label, of, **f):
    c = {"label": label, "value": {"of": of, **{k: v for k, v in f.items() if k != "decimals"}}}
    if "decimals" in f:
        c["decimals"] = f["decimals"]
    return c


PARTS = {
    "name": "Mobilya listesi", "drawings": ["kat2.kcad"], "kinds": ["insert"],
    "scope": {"nested": True, "xrefs": True},
    "columns": [col("Blok", "block"), col("Kod", "attribute", tag="KOD"), col("Açıklama", "joined", field="Açıklama"),
                col("Birim fiyat", "joined", field="Birim fiyat"), col("Adet", "count")],
    "group": True, "sort": [{"column": 0}, {"column": 1}], "total": True,
    "join": {"path": "aciklama.csv", "key": "Blok", "on": 0},
}
METRAJ = {
    "name": "Metraj", "kinds": ["line", "dimension"], "layers": ["Duvar", "Ölçü"],
    "columns": [col("Katman", "layer"), col("Tür", "kind"), col("Uzunluk", "length", decimals=2),
                col("Gerçek ölçü", "measured"), col("Yazılan", "shown"), col("Elle", "overridden"),
                col("Ağırlık", "computed", column=2, factor="7.5", unit="kg")],
    "group": True, "total": True,
}
ROWS = {
    "name": "Sıralar", "kinds": ["insert"], "scope": {"nested": True},
    "columns": [col("Blok", "block"), col("Adet", "count")], "group": True, "sort": [{"column": 0}], "total": True,
}
LEAVES = {
    "name": "Kapı kanatları", "kinds": ["line"], "layers": ["Kapı"], "scope": {"nested": True},
    "columns": [col("Tür", "kind"), col("Katman", "layer"), col("Uzunluk", "length", decimals=2)],
    "total": True,
}
AREAS = {
    "name": "Alanlar", "kinds": ["polygon"], "scope": {"nested": True},
    "columns": [col("Katman", "layer"), col("Tür", "kind"), col("Adet", "count"), col("Alan", "area", decimals=4)],
    "group": True,
}
WHOLE = {
    "name": "Diziler bütün", "kinds": ["insert"], "blocks": ["sandalye"], "scope": {"arrays": "whole", "hidden": True},
    "columns": [col("Tür", "kind"), col("Blok", "block"), col("Katman", "layer"), col("Adet", "count")],
    "group": True, "total": True,
}
XREFS_ONLY = {
    "name": "Referans çizgileri", "kinds": ["line"], "layers": ["Referans"], "scope": {"xrefs": True},
    "columns": [col("Tür", "kind"), col("Uzunluk", "length")],
}
VARIANTS = {
    "name": "Kapı türleri", "kinds": ["insert"], "blocks": ["Kapı"], "drawings": ["kat2.kcad"],
    "columns": [col("Çizim", "drawing"), col("Blok", "block"), col("Tür", "variant"), col("Adet", "count")],
    "group": True, "sort": [{"column": 3, "descending": True}, {"column": 0}],
}
CURRENT_ONLY_PARTS = dict(PARTS, drawings=[], name="Yalnız bu çizim")


def canon(recipe):
    """A recipe as the contract writes it: empty lists and default choices left out."""
    return {k: v for k, v in recipe.items() if v not in ([], {}) and not (k == "current" and v is True)}


def docs_for(recipe):
    out = [("", "Bu çizim", KAT1)] if recipe.get("current", True) else []
    for d in recipe.get("drawings", []):
        assert d == "kat2.kcad"
        out.append((d, "kat2.kcad", KAT2))
    return out


def csv_rows(text):
    return [line.split(";") for line in text.strip("\n").split("\n")]


def reads_for(recipe):
    out = [{"path": d, "digest": KAT2_DIGEST} for d in recipe.get("drawings", [])]
    if recipe.get("join"):
        out.append({"path": recipe["join"]["path"], "digest": CSV_DIGEST})
    return out


def run_output(recipe, stale=None, setup=None):
    docs = [(p, n, setup if (setup and not p) else d) for p, n, d in docs_for(recipe)]
    x = extract(recipe, docs, csv_rows(CSV) if recipe.get("join") else None)
    x["read"] = reads_for(recipe)
    x["revision"] = "$any"
    if stale is not None:
        x["stale"] = stale
    return x


def warnings_for(x):
    n = x["notes"]
    out = []
    if n["hidden"]:
        out.append("extraction_hidden")
    if n["unloaded"]:
        out.append("xref_unloaded")
    if n["readApart"]:
        out.append("xref_read_apart")
    if n["unmatched"]:
        out.append("join_unmatched")
    if n["overrides"]:
        out.append("dimension_overrides")
    if n["tooDeep"]:
        out.append("extraction_too_deep")
    return [{"code": c, "message": "$any"} for c in out]


HELD = dict(canUndo=False, dirty=False, revision="same")


def runs():
    cases = []

    def ok(name, input_, recipe, op="execute", stale=None, setup=None):
        x = run_output(recipe, stale, setup)
        c = dict(name=name, steps=[dict(op=op, input=dict(input_, base=BASE),
                                         result=dict(status="completed", output=x, warnings=warnings_for(x)),
                                         expect=dict(HELD, ids=[e["id"] for e in (setup or KAT1)["entities"]]))])
        if setup:
            c["setup"] = setup
        cases.append(c)
        return x

    def refused(name, input_, code, path, op="execute", setup=None, status="failed"):
        c = dict(name=name, steps=[dict(op=op, input=dict(input_, base=BASE) if "base" not in input_ else input_,
                                         result=dict(status=status, error=dict(code=code, message="$any", path=path)),
                                         expect=dict(HELD))])
        if setup:
            c["setup"] = setup
        cases.append(c)

    parts = ok("iki çizimden parça listesi: iç içe bloklar, diziler, dış tablo, tekil sayım", {"recipe": PARTS}, PARTS)
    # The acceptance's own numbers, beside the reference's general rule.
    by = {r["key"]: r["cells"] for r in parts["rows"]}
    assert by["Masa" + SEP + "M-01"][4] == "3"
    assert by["Sandalye" + SEP + "S-01"][4] == "23" and by["Sandalye" + SEP + "S-02"][4] == "2"
    assert by["Kapı" + SEP][4] == "3" and parts["total"][4] == "31"
    ok("plan aynı sonucu yazısız verir", {"recipe": PARTS}, PARTS, op="plan")
    ok("validate", {"recipe": PARTS}, PARTS, op="validate")
    cases[-1]["steps"][0]["result"]["output"] = None
    only = ok("yalnız açık çizim: dış referans içine bakılır, çizgisi blok değildir", {"recipe": CURRENT_ONLY_PARTS},
              CURRENT_ONLY_PARTS)
    assert not only["notes"]["readApart"]
    metraj = ok("metraj: uzunluk, gerçek ölçü ve elle yazılmış ölçü metni ayrı; toplam yuvarlamadan önce",
                {"recipe": METRAJ}, METRAJ)
    assert metraj["total"][2] == "1.01" and metraj["total"][3] == "7.000" and metraj["total"][6] == "7.56"
    ok("dinamik bloğun gizli kanadı sayılmaz", {"recipe": LEAVES}, LEAVES)
    hidden = dict(LEAVES, scope={"nested": True, "hidden": True}, name="Gizliler de")
    ok("görünmeyen parçalar açıkça seçilince sayılır", {"recipe": hidden}, hidden)
    # A dynamic block's visibility state hides a piece and the array action's copies of it (docs/adr/1131 §2).
    rows = ok("görünürlük durumunun gizlediği parça ve dizi eyleminin kopyaları sayılmaz, sayısı söylenir",
              {"recipe": ROWS}, ROWS, setup=KAT3)
    assert rows["notes"]["hidden"] == 5 and rows["notes"]["hiddenStates"] == 3, rows["notes"]
    by = {r["key"]: r["cells"] for r in rows["rows"]}
    assert by["Sandalye"][1] == "3" and by["Sıra"][1] == "2", by
    every = dict(ROWS, scope={"nested": True, "hidden": True}, name="Sıralar, gizliler de")
    rows = ok("gizli sıranın ve görünürlük durumunun gizlediği kopyalar açıkça seçilince sayılır",
              {"recipe": every}, every, setup=KAT3)
    by = {r["key"]: r["cells"] for r in rows["rows"]}
    assert by["Sandalye"][1] == "7" and by["Sıra"][1] == "3", by
    ok("iç içe alanlar ölçek ve dönüşle", {"recipe": AREAS}, AREAS)
    ok("dizi bütün sayılır, gizli katman dahil", {"recipe": WHOLE}, WHOLE)
    ok("dış referansın içi kaynak kimlikleriyle", {"recipe": XREFS_ONLY}, XREFS_ONLY)
    ok("dinamik varyantlar iki çizimden, adete göre azalan", {"recipe": VARIANTS}, VARIANTS)
    hours = dict(METRAJ, columns=METRAJ["columns"][:3] + [col("Uzunluk", "length", unit="cm", decimals=1)],
                 name="Birimler")
    ok("aynı uzunluk iki birimde", {"recipe": hours}, hours)
    refused("tarif ya da tablo verilmeli", {}, "invalid_input", "recipe")
    refused("ikisi birden verilemez", {"recipe": PARTS, "table": uid(1, 1)}, "invalid_input", "recipe")
    bad = dict(METRAJ, columns=METRAJ["columns"][:4] + [col("Yazılan", "shown"),
                                                          col("Yanlış", "computed", column=4, factor="1")])
    refused("ölçü yazısı sayı gibi çarpılamaz", {"recipe": bad}, "invalid_extraction_recipe",
            "recipe.columns[5].value.column")
    refused("bilinmeyen tür", {"recipe": dict(PARTS, kinds=["wall"])}, "invalid_extraction_recipe", "recipe.kinds[0]")
    refused("miktar olmadan birleştirme", {"recipe": dict(PARTS, columns=PARTS["columns"][:4])},
            "invalid_extraction_recipe", "recipe.group")
    refused("aynı çizim iki kez", {"recipe": dict(PARTS, drawings=["kat2.kcad", "kat2.kcad"])},
            "invalid_extraction_recipe", "recipe.drawings[1]")
    refused("açık çizimin kendi dosyası", {"recipe": PARTS, "drawingPath": "kat2.kcad"}, "drawing_read_twice",
            "recipe.drawings[0]")
    refused("çizim yolunda yok", {"recipe": dict(PARTS, drawings=["yok.kcad"])}, "extraction_source_missing",
            "recipe.drawings[0]")
    refused("dış tablonun anahtarı tekrarlı",
            {"recipe": dict(PARTS, join=dict(PARTS["join"], path="tekrarli.csv"),
                            columns=PARTS["columns"][:3] + [PARTS["columns"][4]])},
            "join_unusable", "recipe.join.key")
    refused("dış tabloda istenen sütun yok",
            {"recipe": dict(PARTS, columns=PARTS["columns"][:2] + [col("Renk", "joined", field="Renk")]
                            + PARTS["columns"][4:])}, "join_unusable", "recipe.columns[2].value.field")
    refused("tablo değil", {"table": uid(1, 14)}, "not_a_table", "table")
    refused("geçersiz kimlik", {"table": "tablo"}, "invalid_uid", "table")
    refused("çizimde olmayan tablo", {"table": uid(1, 999)}, "entity_not_found", "table")
    refused("çizim olmayan dosya", {"recipe": dict(PARTS, drawings=["aciklama.csv"])}, "extraction_source_unreadable",
            "recipe.drawings[0]")
    refused("geçersiz sürüm", {"recipe": PARTS, "expectedRevision": "x"}, "invalid_revision", "expectedRevision")
    unloaded = copy.deepcopy(KAT1)
    xb = next(b for b in unloaded["blocks"] if b["id"] == XREF)
    # No objects: the field is not written at all (a file's writer leaves an empty list out).
    xb["entities"], xb["xref"]["unloaded"] = [], True
    del xb["xref"]["objects"]
    ok("yüklü olmayan dış referansın içine bakılamaz, söylenir", {"recipe": XREFS_ONLY}, XREFS_ONLY, setup=unloaded)
    cases.append(dict(name="eski revizyon", steps=[dict(
        op="execute", input={"recipe": PARTS, "base": BASE, "expectedRevision": "999"},
        result=dict(status="conflict", error=dict(code="revision_conflict", message="$any", path="expectedRevision",
                                                  revision="$current")),
        expect=dict(HELD))]))
    return cases


# ── Tables ────────────────────────────────────────────────────────────────

TABLE_HEIGHT = 0.25


def table_cases():
    cases = []
    ids = [e["id"] for e in KAT1["entities"]]
    n = len(ids)

    def made(name, input_, recipe, extra=None, warn=()):
        x = extract(recipe, docs_for(recipe), csv_rows(CSV) if recipe.get("join") else None)
        cells = cells_of(x)
        source = {"kind": "extraction", "recipe": canon(recipe), "keys": [r["key"] for r in x["rows"]], "copy": cells}
        if reads_for(recipe):
            source["read"] = reads_for(recipe)
        if input_.get("overrides") == "replace":
            source["overrides"] = "replace"
        aligns = ["right" if quantity(c) else "left" for c in recipe["columns"]]
        fields = {"cells": cells, "header": True, "aligns": aligns, "source": source, "height": TABLE_HEIGHT,
                  "p": input_["at"], "layerId": input_.get("layerId", "a")}
        step = dict(op="execute", input=dict(input_, recipe=recipe, base=BASE, height=TABLE_HEIGHT),
                    result=dict(status="completed",
                                output=dict(uid="$any", rows=len(x["rows"]), occurrences=x["occurrences"],
                                            notes=x["notes"], revision="$any"),
                                warnings=[{"code": w, "message": "$any", "path": "layerId"} for w in warn] + warnings_for(x)),
                    expect=dict(ids=ids + [n + 1], canUndo=True, dirty=True, revision="changed",
                                fields={str(n + 1): fields}))
        steps = [step]
        if extra:
            steps.extend(extra)
        cases.append(dict(name=name, steps=steps))

    made("parça listesi tablo olur; tarif ve son kopya tabloyla saklanır", {"at": P(30, 0)}, PARTS,
         extra=[dict(op="undo", expect=dict(ids=ids, canUndo=False, canRedo=True, dirty=True))])
    made("metraj tablosu, katmanı verilmiş", {"at": P(30, -10), "layerId": "duvar"}, METRAJ)
    made("dosyayla değiştir kuralı saklanır", {"at": P(30, 10), "overrides": "replace"}, AREAS)
    made("yalnız açık çizimin parça listesi", {"at": P(30, 20)}, CURRENT_ONLY_PARTS)
    made("kapı kanatları satır satır, yollarıyla", {"at": P(30, 30)}, LEAVES)
    made("gizli kanatlar da", {"at": P(30, 40)}, dict(LEAVES, scope={"nested": True, "hidden": True},
                                                       name="Gizliler de"))
    made("dizi bütün", {"at": P(30, 50)}, WHOLE)
    made("dış referansın çizgileri", {"at": P(30, 60)}, XREFS_ONLY)
    made("dinamik varyantlar", {"at": P(30, 70)}, VARIANTS)
    made("santimetre ve metre yan yana", {"at": P(30, 80)},
         dict(METRAJ, columns=METRAJ["columns"][:3] + [col("Uzunluk", "length", unit="cm", decimals=1)],
              name="Birimler"))
    made("dönüm ve hektar", {"at": P(30, 90)},
         dict(AREAS, columns=AREAS["columns"] + [col("Alan", "area", unit="donum", decimals=6),
                                                 col("Alan", "area", unit="ha", decimals=7)], name="Alan birimleri"))
    made("gizli katmana yazılan tablo uyarır", {"at": P(40, 0), "layerId": "gizli"}, WHOLE, warn=("layer_hidden",))
    made("kilometre", {"at": P(30, 100)}, dict(XREFS_ONLY, columns=[col("Uzunluk", "length", unit="km", decimals=4)],
                                             name="Km"))
    for op in ("plan", "validate"):
        x = extract(PARTS, docs_for(PARTS), csv_rows(CSV))
        out = None if op == "validate" else dict(table="$any", rows=len(x["rows"]), occurrences=x["occurrences"],
                                                   notes=x["notes"], revision="$any")
        cases.append(dict(name=f"{op} tablo yazmaz", steps=[dict(
            op=op, input={"recipe": PARTS, "at": P(30, 0), "base": BASE, "height": TABLE_HEIGHT},
            result=dict(status="completed", output=out, warnings=warnings_for(x)), expect=dict(HELD, ids=ids))]))

    def refused(name, input_, code, path):
        cases.append(dict(name=name, steps=[dict(op="execute", input=dict({"at": P(0, 0), "base": BASE}, **input_),
                                                 result=dict(status="failed",
                                                             error=dict(code=code, message="$any", path=path)),
                                                 expect=dict(HELD, ids=ids))]))

    refused("hiç nesne bulmayan tarif tablo yazmaz", {"recipe": dict(PARTS, blocks=["Yok"])}, "extraction_empty",
            "recipe")
    refused("kilitli katmana yazılmaz", {"recipe": PARTS, "layerId": "kilitli"}, "layer_locked", "layerId")
    refused("bozuk tarif", {"recipe": dict(PARTS, name=" ")}, "invalid_extraction_recipe", "recipe.name")
    refused("okunan çizim yok", {"recipe": dict(PARTS, drawings=["yok.kcad"])}, "extraction_source_missing",
            "recipe.drawings[0]")
    refused("açık çizimin kendi dosyası", {"recipe": PARTS, "drawingPath": "kat2.kcad"}, "drawing_read_twice",
            "recipe.drawings[0]")
    refused("çizim olmayan dosya", {"recipe": dict(PARTS, drawings=["aciklama.csv"])}, "extraction_source_unreadable",
            "recipe.drawings[0]")
    refused("tekrarlı anahtarlı dış tablo",
            {"recipe": dict(PARTS, join=dict(PARTS["join"], path="tekrarli.csv"),
                            columns=PARTS["columns"][:3] + [PARTS["columns"][4]])}, "join_unusable", "recipe.join.key")
    refused("olmayan katman", {"recipe": PARTS, "layerId": "yok"}, "layer_not_found", "layerId")
    refused("katman grubu", {"recipe": PARTS, "layerId": "grup"}, "not_a_layer", "layerId")
    refused("geçersiz sürüm", {"recipe": PARTS, "expectedRevision": "x"}, "invalid_revision", "expectedRevision")
    cases.append(dict(name="eski sürüm", steps=[dict(
        op="execute", input={"recipe": PARTS, "at": P(0, 0), "base": BASE, "expectedRevision": "999"},
        result=dict(status="conflict", error=dict(code="revision_conflict", message="$any", path="expectedRevision",
                                                  revision="$current")),
        expect=dict(HELD, ids=ids))]))
    refused("yazı yüksekliği sıfırdan büyük olmalı", {"recipe": PARTS, "height": 0.0}, "invalid_table", "height")
    cases.append(dict(name="köşesi sonlu olmalı", steps=[dict(
        op="execute", input={"recipe": PARTS, "at": P(0, 0), "base": BASE}, nonFinite={"at.x": "Infinity"},
        result=dict(status="failed", error=dict(code="not_finite", message="$any", path="at.x")),
        expect=dict(HELD, ids=ids))]))
    refused("dış tablo yok", {"recipe": dict(PARTS, join=dict(PARTS["join"], path="yok.csv"))},
            "extraction_source_missing", "recipe.join.path")
    return cases


# ── Tabloyu güncelle ──────────────────────────────────────────────────────


def table_entity(i, recipe, cells, keys, copy_cells, reads, overrides=None, formulas=None, fields=None):
    m = len(cells[0])
    t = {"kind": "table", "id": i, "layerId": "a", "attrs": {}, "p": P(30, 0), "rotation": 0.0,
         "height": TABLE_HEIGHT, "rows": [0.5] * len(cells), "columns": [3.0] * m, "cells": cells, "header": True,
         "aligns": ["right" if j < len(recipe["columns"]) and quantity(recipe["columns"][j]) else "left"
                    for j in range(m)],
         "source": {"kind": "extraction", "recipe": canon(recipe), "keys": keys, "copy": copy_cells}}
    if reads:
        t["source"]["read"] = reads
    if overrides:
        t["source"]["overrides"] = overrides
    if formulas:
        t["formulas"] = formulas
    if fields:
        t["fields"] = fields
    return t


def refresh_cases():
    """A parts list written before the sources changed: no Sandalye S-02 row yet, one Sandalye S-01 fewer, the old
    description “Sandalye”; the user wrote the Masa's description by hand, a Not column and a Tutar column (Birim fiyat
    × Adet) with its total over every data row."""
    now = extract(PARTS, docs_for(PARTS), csv_rows(CSV))
    fresh = cells_of(now)
    keys = [r["key"] for r in now["rows"]]
    s01, s02, masa = "Sandalye" + SEP + "S-01", "Sandalye" + SEP + "S-02", "Masa" + SEP + "M-01"
    # The old extraction: the S-02 row not there yet, S-01 one fewer, the total so too.
    old_keys = [k for k in keys if k != s02]
    old = [fresh[0]] + [fresh[keys.index(k) + 1][:] for k in old_keys] + [fresh[-1][:]]
    r_s01 = old_keys.index(s01) + 1
    r_masa = old_keys.index(masa) + 1
    old[r_s01][4] = str(int(old[r_s01][4]) - 1)
    old[r_s01][2] = "Sandalye"
    gone_count = int(fresh[keys.index(s02) + 1][4])
    old[-1][4] = str(int(old[-1][4]) - 1 - gone_count)
    reads_old = [{"path": "kat2.kcad", "digest": "0" * 64}, {"path": "aciklama.csv", "digest": "1" * 64}]

    def user(cells_, keys_):
        out = []
        for r, row in enumerate(cells_):
            if r == 0:
                extra = ["Not", "Tutar"]
            elif r <= len(keys_) and keys_[r - 1] == s01:
                extra = ["acil", ""]
            else:
                extra = ["", ""]
            out.append(row + extra)
        return out

    def formulas_for(n_rows, data_rows):
        """Tutar = Birim fiyat × Adet on the rows `data_rows` (zero-based), and its total over rows 2 to n−1."""
        f = [{"row": r, "col": 6, "formula": f"=D{r + 1}*E{r + 1}"} for r in data_rows]
        f.append({"row": n_rows - 1, "col": 6, "formula": f"=TOPLA(G2:G{n_rows - 1})"})
        return f

    hand = user(old, old_keys)
    hand[r_masa][2] = "Toplantı masası"
    old_formulas = formulas_for(len(old), range(1, len(old) - 1))
    setup = copy.deepcopy(KAT1)
    tid = len(setup["entities"]) + 1
    setup["entities"].append(table_entity(tid, PARTS, hand, old_keys, old, reads_old, formulas=old_formulas))
    setup["uids"].append(uid(1, tid))
    ids = [e["id"] for e in setup["entities"]]
    # After: the S-02 row in its sorted place, the old rows' formulas following their keys, the total's span over every
    # data row now; the Tutar cells are the formula engine's (its own reference), not this script's.
    new_row = {k: keys.index(k) + 1 for k in keys}
    expected = user(fresh, keys)
    expected[new_row[masa]][2] = "Toplantı masası"
    for row in expected[1:]:
        row[6] = "$any"
    new_formulas = [{"row": new_row[old_keys[f["row"] - 1]], "col": 6,
                     "formula": "=D{0}*E{0}".format(new_row[old_keys[f["row"] - 1]] + 1)}
                    for f in old_formulas[:-1]]
    new_formulas.append({"row": len(fresh) - 1, "col": 6, "formula": f"=TOPLA(G2:G{len(fresh) - 1})"})
    refresh = {"operation": {"kind": "refresh", "uids": [uid(1, tid)], "base": BASE}}
    tables = "cad.tables.edit"

    def report(status, kept=0, replaced=0, changed=False):
        return dict(changed=[uid(1, tid)] if changed else [],
                    report=[dict(uid=uid(1, tid), status=status, kept=kept, replaced=replaced)], revision="$any")

    cases = []
    source_after = dict(setup["entities"][-1]["source"], read=reads_for(PARTS), copy=fresh, keys=keys)
    cases.append(dict(name="kaynak değişti: yeni satır, adet ve açıklama; elle düzeltme, kullanıcı sütunları ve "
                           "formüller anahtarla korunur", setup=setup, steps=[dict(
                          op="execute", command=tables, input=refresh,
                          result=dict(status="completed", output=report("updated", kept=1, changed=True),
                                      warnings=warnings_for(now)),
                          expect=dict(ids=ids, canUndo=True, dirty=True, revision="changed",
                                      fields={str(tid): {"cells": expected, "formulas": new_formulas,
                                                         "source": source_after}})),
                          dict(op="undo", expect=dict(ids=ids, canUndo=False, canRedo=True, dirty=True,
                                                      fields={str(tid): {"cells": hand,
                                                                         "formulas": old_formulas}}))]))
    cases.append(dict(name="ikinci yenileme değişiklik bulmaz", setup=setup, steps=[
        dict(op="execute", command=tables, input=refresh, result=dict(status="completed", output="$any", warnings="$any"),
             expect=dict(revision="changed")),
        dict(op="execute", command=tables, input=refresh,
             result=dict(status="completed", output=report("unchanged"), warnings=warnings_for(now)),
             expect=dict(revision="same"))]))
    # A hand edit on a cell whose source changed too: the table is kept.
    clash = copy.deepcopy(setup)
    clash["entities"][-1]["cells"][r_s01][2] = "Kırmızı sandalye"
    kept_warning = [{"code": "refresh_kept", "message": "$any"}]
    cases.append(dict(name="kaynak ve kullanıcı aynı hücreyi değiştirdi: tablo korunur", setup=clash, steps=[dict(
        op="execute", command=tables, input=refresh,
        result=dict(status="completed", output=report("unreadable"), warnings=kept_warning),
        expect=dict(HELD, ids=ids))]))
    # The same with the rule to write over: the source's words win, and are counted.
    over = copy.deepcopy(clash)
    over["entities"][-1]["source"]["overrides"] = "replace"
    cases.append(dict(name="dosyayla değiştir kuralında kaynak yazılır ve sayılır", setup=over, steps=[dict(
        op="execute", command=tables, input=refresh,
        result=dict(status="completed", output=report("updated", kept=1, replaced=1, changed=True),
                    warnings=warnings_for(now)),
        expect=dict(ids=ids, canUndo=True, dirty=True, revision="changed", fields={str(tid): {"cells": expected}}))]))
    # A row the user inserted: the rows cannot be matched by their keys.
    inserted = copy.deepcopy(setup)
    t = inserted["entities"][-1]
    t["cells"].insert(1, [""] * len(t["cells"][0]))
    t["rows"].insert(1, 0.5)
    # Its formulas dropped: no field, as a file writes none (an empty list is not written).
    del t["formulas"]
    cases.append(dict(name="kullanıcı satır eklediyse tablo korunur", setup=inserted, steps=[dict(
        op="execute", command=tables, input=refresh,
        result=dict(status="completed", output=report("unreadable"), warnings=kept_warning),
        expect=dict(HELD, ids=ids))]))
    # A row with the user's words that the sources no longer give: kept under Koru.
    vanish = copy.deepcopy(setup)
    vt = vanish["entities"][-1]
    vt["source"]["recipe"] = canon(dict(PARTS, blocks=["Masa", "Kapı"]))
    cases.append(dict(name="kullanıcı notlu satır kaynaktan kalkınca tablo korunur", setup=vanish, steps=[dict(
        op="execute", command=tables, input=refresh,
        result=dict(status="completed", output=report("unreadable"), warnings=kept_warning),
        expect=dict(HELD, ids=ids))]))
    for name, recipe, code in (
            ("okunan dosya çizim değilse tablo korunur", dict(PARTS, drawings=["aciklama.csv"]),
             "extraction_source_unreadable"),
            ("dış tablo artık eşlenemiyorsa tablo korunur",
             dict(PARTS, join=dict(PARTS["join"], path="tekrarli.csv"),
                  columns=PARTS["columns"][:3] + [PARTS["columns"][4]]), "join_unusable")):
        broken = copy.deepcopy(setup)
        broken["entities"][-1]["source"]["recipe"] = canon(recipe)
        if code == "join_unusable":
            bt = broken["entities"][-1]
            keep = [0, 1, 2, 4, 5, 6]
            bt["cells"] = [[r[j] for j in keep] for r in bt["cells"]]
            bt["source"]["copy"] = [[r[j] for j in (0, 1, 2, 4)] for r in bt["source"]["copy"]]
            bt["columns"] = bt["columns"][:6]
            bt["aligns"] = [bt["aligns"][j] for j in keep]
            del bt["formulas"]
        cases.append(dict(name=name, setup=broken, steps=[dict(
            op="execute", command=tables, input=refresh,
            result=dict(status="completed", output=report("unreadable"), warnings=[{"code": code, "message": "$any"}]),
            expect=dict(HELD, ids=ids))]))
    # A drawing the recipe reads gone: Missing, the table kept.
    gone = copy.deepcopy(setup)
    gone["entities"][-1]["source"]["recipe"] = canon(dict(PARTS, drawings=["yok.kcad"]))
    cases.append(dict(name="okunan çizim yoksa tablo olduğu gibi kalır", setup=gone, steps=[dict(
        op="execute", command=tables, input=refresh,
        result=dict(status="completed", output=report("missing"),
                    warnings=[{"code": "extraction_source_missing", "message": "$any"}]),
        expect=dict(HELD, ids=ids))]))
    # cad.extract.run says whether such a table is current, writing nothing.
    run_stale = dict(name="tablo güncel değil: kaynakları değişmiş", setup=setup, steps=[dict(
        op="execute", input={"table": uid(1, tid), "base": BASE},
        result=dict(status="completed", output=run_output(PARTS, stale=True), warnings=warnings_for(now)),
        expect=dict(HELD, ids=ids))])
    plain = copy.deepcopy(KAT1)
    pid = len(plain["entities"]) + 1
    plain["entities"].append({"kind": "table", "id": pid, "layerId": "a", "attrs": {}, "p": P(0, 0), "rotation": 0.0,
                              "height": 0.25, "rows": [0.5], "columns": [1.0], "cells": [["A"]]})
    plain["uids"].append(uid(1, pid))
    run_plain = dict(name="tarifsiz tablo", setup=plain, steps=[dict(
        op="execute", input={"table": uid(1, pid), "base": BASE},
        result=dict(status="failed", error=dict(code="not_an_extraction", message="$any", path="table")),
        expect=dict(HELD))])
    cases += table_model_cases(plain, pid)
    return cases, [run_stale, run_plain]


def table_model_cases(plain, pid):
    """Tabloyu böl and cells of several lines (docs/adr/1030 §7) through `cad.tables.edit` on a plain table."""
    t = plain["entities"][-1]
    t.update(rows=[0.5, 0.5, 0.5], columns=[2.0, 2.0], cells=[["Ad", "Not"], ["A", "1"], ["B", "2"]], header=True)
    ids = [e["id"] for e in plain["entities"]]
    tables = "cad.tables.edit"
    h = F(t["height"])

    def edit(*edits):
        return {"operation": {"kind": "edit", "uid": uid(1, pid), "edits": list(edits)}}

    def done(name, edits, fields, steps_before=()):
        steps = list(steps_before) + [dict(
            op="execute", command=tables, input=edit(*edits),
            result=dict(status="completed", output=dict(changed=[uid(1, pid)], revision="$any"), warnings=[]),
            expect=dict(ids=ids, canUndo=True, dirty=True, revision="changed", fields={str(pid): fields}))]
        return dict(name=name, setup=plain, steps=steps)

    def refused(name, edits, path):
        return dict(name=name, setup=plain, steps=[dict(
            op="execute", command=tables, input=edit(*edits),
            result=dict(status="failed", error=dict(code="invalid_table", message="$any", path=path)),
            expect=dict(HELD, ids=ids))])

    breaks = {"height": 1.0, "gap": 0.5}
    lines = "Ahşap\nkapı"
    return [
        done("tablo parçalara bölünür, başlık her parçada", [{"kind": "breaks", "breaks": breaks}], {"breaks": breaks}),
        done("başlık yalnız ilk parçada", [{"kind": "breaks", "breaks": dict(breaks, once=True)}],
             {"breaks": dict(breaks, once=True)}),
        done("bölme kaldırılır", [{"kind": "breaks", "breaks": None}], {"breaks": None}, steps_before=[dict(
            op="execute", command=tables, input=edit({"kind": "breaks", "breaks": breaks}),
            result=dict(status="completed", output="$any", warnings=[]), expect=dict(revision="changed"))]),
        refused("parça derinliği sıfırdan büyük olmalı", [{"kind": "breaks", "breaks": {"height": 0.0, "gap": 0.0}}],
                "operation.edits[0]"),
        refused("parça aralığı eksi olamaz", [{"kind": "breaks", "breaks": {"height": 1.0, "gap": -1.0}}],
                "operation.edits[0]"),
        done("hücrede satır sonu", [{"kind": "setCell", "row": 1, "col": 1, "words": lines}],
             {"cells": [["Ad", "Not"], ["A", lines], ["B", "2"]]}),
        refused("bir hücre en çok 20 satırdır", [{"kind": "setCell", "row": 1, "col": 1, "words": "\n".join("x" * 21)}],
                "operation.cells"),
        refused("satır sonundan başka denetim karakteri olmaz", [{"kind": "setCell", "row": 1, "col": 1,
                                                                    "words": "a\nb\tc"}], "operation.cells"),
        done("yazıya sığdır satırları satır sayısınca derinleştirir",
             [{"kind": "setCell", "row": 1, "col": 1, "words": lines},
              {"kind": "fit", "from": 0, "count": 2, "rows": True}],
             {"rows": [float(2 * h), float((2 + LINE) * h), float(2 * h)]}),
    ]


def fixture(command, title, note, setup, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": setup, "cases": cases}


def files():
    refreshes, more_runs = refresh_cases()
    note = "ADR 1030. Independent expectations (scripts/fixtures/extract_command_cases.py, no KentOS code)."
    out = {
        SRC / "kat2.kcad": KAT2_BYTES,
        SRC / "kat2.json": (json.dumps(KAT2, ensure_ascii=False, indent=1) + "\n").encode(),
        SRC / "aciklama.csv": CSV.encode(),
        SRC / "tekrarli.csv": CSV_REPEATED.encode(),
        OUT / "cad.extract.run.json": fixture("cad.extract.run", "Veri çıkarımı", note, KAT1, runs() + more_runs),
        OUT / "cad.extract.table.json": fixture("cad.extract.table", "Veri çıkarımı tablosu ve güncellemesi", note,
                                               KAT1, table_cases() + refreshes),
    }
    return {p: (v if isinstance(v, bytes) else (json.dumps(v, ensure_ascii=False, indent=1) + "\n").encode())
            for p, v in out.items()}


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--check", action="store_true")
    args = cli.parse_args()
    stale = []
    for path, data in files().items():
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    if stale:
        print("stale:", *stale, sep="\n  ")
        sys.exit(1)
    print("ok" if args.check else "written")


if __name__ == "__main__":
    main()
