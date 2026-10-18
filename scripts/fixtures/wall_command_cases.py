#!/usr/bin/env python3
"""The wall commands' shared cases (docs/adr/0339 §5), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.walls.{create,convert,edit}.json; `--check` compares and writes nothing. Whether
a wall's faces can be drawn is the drawing reference's (scripts/fixtures/wall_cases.py: the path offset to each face).

The rules, as ADR 0339 §5 gives them:
- a wall is given by its path (vertices, one bulge per edge, closed or not), width, justification, offset, style,
  ends, height and architectural information; the document keeps one bulge per vertex (an open path's last 0) and
  none when every edge is straight;
- a style is found by its id, then its name (case aside), the project's first, then KentOS's own (taken into the
  project, marked “(KentOS)” when a project style has its name); a wall of a style takes its components, ends and
  fill; its width is the one given, else the components' sum when they fix it, else the style's new width; a plain
  wall's is the one given, else 0.2 m;
- `cad.walls.create`: no wall (`no_objects`); each wall in order: its coordinates, bulges, their count, two vertices
  in a row at one place, its lengths finite, its style, the wall's rule (the file's), its faces drawable, its level
  the project's; then the line weight, the revision, the layer, the attributes;
- `cad.walls.convert`: the ids, the objects, those on locked layers left out (all: refused); each mode: a line,
  polyline, area's ring, arc (its bulge tan(sweep/4)) or circle (two half circles) a wall each, another kind left;
  pairs mode: straight lines paired, nearest first (by distance, then the longer overlap, then the first's and the
  second's places), directions within `angle` degrees, at most `maxWidth` apart, overlapping over more than 1 µm; the
  wall along the first over the overlap, moved half the distance towards the second, that wide, centred; a style
  only when its components fit; a source the wall does not cover whole is kept (`source_kept`);
- `cad.walls.edit`: the ids, the objects, walls only, something to set, the numbers finite, the style; each wall
  changed (the style's fields first, then the ones given) and checked as a new one is; the revision; a wall on a
  locked layer refuses the whole change.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_cases  # noqa: E402  (the drawing's reference: a wall's faces drawn, or not)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
HALF_PI = 1.5707963267948966


def num(x):
    """A number as Rust's `{}` writes an f64: integral values without a point, the others as the shortest repr."""
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


def P(x, y):
    return {"x": float(x), "y": float(y)}


PERDE = {"id": "proje-perde-20", "name": "Perde 20",
         "components": [{"name": "Betonarme", "material": "Betonarme", "thickness": 0.2,
                         "hatch": {"pattern": "SOLID", "scale": 1.0, "angle": 0.0, "color": "#A6A6A6"}, "detail": "standard"}],
         "fill": "#3C3C3C"}
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def base(id, layer_id, **extra):
    return {"id": id, "layerId": layer_id, "attrs": {}, **extra}


def wall(id, layer_id, pts, width, justify="center", **extra):
    return {"kind": "wall", **base(id, layer_id), "pts": pts, "width": width, "justify": justify, **extra}


SETUP_ENTITIES = [
    {"kind": "line", **base(1, "cizim"), "a": P(0, 0), "b": P(5, 0)},
    {"kind": "line", **base(2, "cizim"), "a": P(0, 0.25), "b": P(5, 0.25)},
    {"kind": "line", **base(3, "cizim"), "a": P(6, 0), "b": P(10, 0)},
    {"kind": "line", **base(4, "cizim"), "a": P(6.5, 0.2), "b": P(9, 0.2)},
    {"kind": "line", **base(5, "cizim"), "a": P(0, 5), "b": P(4, 5)},
    {"kind": "polyline", **base(6, "cizim"), "pts": [P(0, 10), P(4, 10), P(4, 13)], "bulges": [0.0, 0.3, 0.0]},
    {"kind": "polygon", **base(7, "cizim"), "pts": [P(10, 10), P(14, 10), P(14, 13), P(10, 13)]},
    {"kind": "arc", **base(8, "cizim"), "c": P(20, 0), "r": 3.0, "a0": 0.0, "a1": HALF_PI},
    {"kind": "circle", **base(9, "cizim"), "c": P(30, 0), "r": 2.0},
    {"kind": "text", **base(10, "cizim"), "p": P(0, 15), "text": "Not", "height": 0.5, "rotation": 0.0},
    {"kind": "line", **base(11, "kilitli"), "a": P(12, 20), "b": P(16, 20)},
    wall(12, "duvar", [P(0, 20), P(6, 20)], 0.2),
    wall(13, "duvar", [P(8, 20), P(14, 20), P(14, 24)], 0.2, style="proje-perde-20", components=PERDE["components"],
         fill="#3C3C3C", arch={"level": "zemin"}),
    wall(14, "kilitli", [P(20, 20), P(24, 20)], 0.2),
    {"kind": "polyline", **base(15, "cizim"), "pts": [P(20, 10), P(24, 10)]},
    {"kind": "line", **base(16, "cizim"), "a": P(20, 10.15), "b": P(24, 10.15)},
    {"kind": "line", **base(17, "cizim"), "a": P(0, 0.5), "b": P(5, 0.5)},
    {"kind": "line", **base(18, "cizim"), "a": P(30, 10), "b": P(34, 10)},
    {"kind": "line", **base(19, "cizim"), "a": P(30, 10.2), "b": P(34, 10.2 + 4 * math.tan(math.radians(0.5)))},
    {"kind": "line", **base(20, "cizim"), "a": P(40, 10), "b": P(44, 10)},
    {"kind": "line", **base(21, "cizim"), "a": P(40, 10.2), "b": P(44, 10.2 + 4 * math.tan(math.radians(2.0)))},
    wall(22, "duvar", [P(0, 30), P(4, 30)], 0.2, bulges=[1.0, 0.0]),
]
IDS = [e["id"] for e in SETUP_ENTITIES]
FIRST_NEW = 23

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Duvarlar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "wallStyles": [PERDE], "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [
        layer("duvar", "Duvar"),
        layer("cizim", "Çizim"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
        layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")]),
    ],
    "activeLayer": "duvar",
    "styles": {"items": [], "categories": []},
    "entities": SETUP_ENTITIES,
}
LAYERS = {"duvar": "Duvar", "cizim": "Çizim", "kilitli": "Kilitli katman", "gizli": "Gizli katman", "sinir": "Sınır"}

# ── KentOS's own wall styles (ADR 0339 §2's table), metres ──────────────────────────────────────────────────────


def comp(name, thickness, detail, hatch=None):
    c = {"name": name, "material": name}
    if thickness is not None:
        c["thickness"] = thickness
    if hatch:
        pattern, scale, color = hatch
        c["hatch"] = {"pattern": pattern, "scale": scale, "angle": 0.0}
        if color:
            c["hatch"]["color"] = color
    c["detail"] = detail
    return c


def plaster(t, name):
    return comp(name, t, "detailed")


BUILTIN = [
    {"id": "kentos-dis-duvar-25", "name": "Dış duvar 25",
     "components": [plaster(0.02, "İç sıva"), comp("Tuğla", None, "standard", ("ANSI31", 1.0, None)),
                    comp("Isı yalıtımı", 0.05, "standard", ("ANSI37", 0.5, None)), plaster(0.03, "Dış sıva")],
     "fill": "#5A5A5A", "width": 0.25},
    {"id": "kentos-ic-duvar-15", "name": "İç duvar 15",
     "components": [plaster(0.02, "Sıva"), comp("Tuğla", None, "standard", ("ANSI31", 1.0, None)), plaster(0.02, "Sıva")],
     "fill": "#5A5A5A", "width": 0.15},
    {"id": "kentos-gazbeton-20", "name": "Gazbeton 20",
     "components": [plaster(0.02, "Sıva"), comp("Gazbeton", None, "standard", ("DOTS", 1.0, None)), plaster(0.02, "Sıva")],
     "fill": "#5A5A5A", "width": 0.20},
    {"id": "kentos-betonarme-25", "name": "Betonarme perde 25",
     "components": [comp("Betonarme", None, "standard", ("SOLID", 1.0, "#A6A6A6"))], "fill": "#3C3C3C", "width": 0.25},
    {"id": "kentos-bolme-10", "name": "Bölme duvar 10",
     "components": [plaster(0.0125, "Alçı levha"), comp("Yalıtım", None, "detailed", ("ANSI37", 0.5, None)), plaster(0.0125, "Alçı levha")],
     "width": 0.10},
]


def find_style(project, key):
    folded = key.strip().lower()
    for table, builtin in ((project, False), (BUILTIN, True)):
        for s in table:
            if s["id"] == key:
                return s, builtin
        for s in table:
            if s["name"].lower() == folded:
                return s, builtin
    return None, False


def no_style(project, key, path):
    names = [s["name"] for s in project]
    for s in BUILTIN:
        if s["name"] not in names:
            names.append(s["name"])
    return refusal("style_not_found", f"“{key}” adında ya da kimliğinde duvar stili yok. Var olanlar: {', '.join(names)}.", path)


def fixed_width(comps):
    if comps and all("thickness" in c for c in comps):
        total = 0.0
        for c in comps:
            total += c["thickness"]
        return total
    return None


def new_width(s):
    f = fixed_width(s["components"])
    if f is not None:
        return f
    if not s["components"]:
        return s.get("width", 0.2)
    if "width" in s:
        return s["width"]
    total = 0.0
    for c in s["components"]:
        total += c.get("thickness", 0.0)
    return total + 0.1


# ── A wall's rule (the file's) ──────────────────────────────────────────────────────────────────────────────────


def thicknesses_fit(width, comps):
    fixed = 0.0
    for c in comps:
        if "thickness" in c:
            fixed += c["thickness"]
    rest = [c for c in comps if "thickness" not in c]
    slack = 1e-9 * max(width, 1.0)
    if not rest:
        return abs(fixed - width) <= slack, fixed, None
    return fixed < width - slack, fixed, rest[0]["name"]


def wall_problem(w):
    """The wall's problem as the file's rule says it: (field, words), or None."""
    pts = w["pts"]
    n = len(pts)
    if n < 2:
        return "pts", f"Duvarın yolu en az iki noktadır; {n} verildi."
    curved = any(x != 0 for x in w.get("bulges", []))
    if w.get("closed") and n < 3 and not curved:
        return "pts", "Kapalı duvar düz kenarlarla en az üç noktadır."
    width = w["width"]
    if not (math.isfinite(width) and 0 < width <= 100):
        return "width", f"Duvarın genişliği {num(width)}; sıfırdan büyük, en çok 100 m olmalı."
    comps = w.get("components", [])
    if comps:
        fits, fixed, rest = thicknesses_fit(width, comps)
        if not fits:
            if rest is not None:
                return "width", (f"Duvarın genişliği {num(width)} m, “{rest}” dışındaki bileşenler {num(fixed)} m; kalan bileşene yer "
                                 "kalmıyor. Duvarı genişletin ya da bileşenleri inceltin.")
            return "width", (f"Duvarın genişliği {num(width)} m, bileşenlerin toplamı {num(fixed)} m: kalınlığı verilmeyen bileşen "
                             "olmadığından genişlik toplama eşit olmalı.")
    if "height" in w and not w["height"] > 0:
        return "height", f"Duvarın yüksekliği {num(w['height'])}; sıfırdan büyük olmalı."
    return None


def drawable(w):
    case = {"pts": [[p["x"], p["y"]] for p in w["pts"]], "width": w["width"], "justify": w["justify"]}
    for key in ("closed", "offset", "components"):
        if key in w:
            case[key] = w[key]
    if "bulges" in w:
        case["bulges"] = w["bulges"]
    return wall_cases.lines(case) is not None


COLLAPSE = "Duvar çizilemiyor: Öteleme mesafesi bir yay parçasının yarıçapından büyük. Duvarı inceltin ya da yayı büyütün."


def level_known(settings, w, path):
    level = w.get("arch", {}).get("level")
    if level is None or any(lv["id"] == level for lv in settings["arch"]["levels"]):
        return None
    return refusal("level_not_found", f"Projede “{level}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", path)


def finite_point(p, i, path):
    for axis, name in (("x", "doğu (Y)"), ("y", "kuzey (X)")):
        if not math.isfinite(p[axis]):
            return refusal("not_finite", f"{i + 1}. noktanın {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"{path}.{axis}")
    return None


def not_finite(name, path):
    return refusal("not_finite", f"{name} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", path)


def path_numbers(pts, bulges, closed, prefix):
    for i, p in enumerate(pts):
        r = finite_point(p, i, f"{prefix}.pts[{i}]")
        if r:
            return r
    if bulges is not None:
        for i, x in enumerate(bulges):
            if not math.isfinite(x):
                return not_finite(f"{i + 1}. büküm", f"{prefix}.bulges[{i}]")
        edges = len(pts) if closed else len(pts) - 1
        if len(bulges) != edges:
            return refusal("invalid_wall", f"Her kenarın bir bükümü olur: {edges} kenar, {len(bulges)} büküm (açık yolda noktalardan bir eksik, kapalıda nokta kadar).", f"{prefix}.bulges")
    n = len(pts)
    edges = n if closed else n - 1
    for i in range(edges):
        a, b = pts[i], pts[(i + 1) % n]
        if a["x"] == b["x"] and a["y"] == b["y"] and n > 1:
            return refusal("coincident_points", f"{i + 1}. ve {(i + 1) % n + 1}. nokta aynı yerde: duvarın kenarı uzunluksuz olur. Yinelenen noktayı çıkarın.", f"{prefix}.pts[{(i + 1) % n}]")
    return None


def build(settings, nw, prefix, added):
    """A wall as the user gave it: its stored fields, or the refusal."""
    r = path_numbers(nw["pts"], nw.get("bulges"), nw.get("closed", False), prefix)
    if r:
        return r
    for key, name in (("width", "Genişlik"), ("offset", "Öteleme"), ("height", "Yükseklik")):
        if key in nw and not math.isfinite(nw[key]):
            return not_finite(name, f"{prefix}.{key}")
    style = None
    if "style" in nw:
        style, builtin = find_style(settings["wallStyles"], nw["style"])
        if style is None:
            return no_style(settings["wallStyles"], nw["style"], f"{prefix}.style")
        if builtin and not any(a["id"] == style["id"] for a in added):
            added.append(style)
    w = {"pts": nw["pts"]}
    b = nw.get("bulges")
    if b is not None and any(x != 0 for x in b):
        w["bulges"] = list(b) + ([] if nw.get("closed") else [0.0])
    if nw.get("closed"):
        w["closed"] = True
    if style:
        width = nw["width"] if "width" in nw else (fixed_width(style["components"]) if fixed_width(style["components"]) is not None else new_width(style))
    else:
        width = nw.get("width", 0.2)
    w["width"] = width
    w["justify"] = nw.get("justify", "center")
    if nw.get("offset", 0.0) != 0.0:
        w["offset"] = nw["offset"]
    if style:
        w["style"] = style["id"]
        w["components"] = copy.deepcopy(style["components"])
    caps = nw.get("caps", style.get("caps", "closed") if style else "closed")
    if caps == "open":
        w["caps"] = "open"
    if style and "fill" in style:
        w["fill"] = style["fill"]
    if "height" in nw:
        w["height"] = nw["height"]
    if nw.get("arch"):
        w["arch"] = nw["arch"]
    p = wall_problem(w)
    if p:
        return refusal("invalid_wall", p[1], f"{prefix}.{p[0]}")
    if not drawable(w):
        return refusal("wall_collapses", COLLAPSE, f"{prefix}.pts")
    r = level_known(settings, w, f"{prefix}.arch.level")
    if r:
        return r
    return w


def is_refusal(x):
    return isinstance(x, dict) and "status" in x


def layer_check(layer_id, path="layerId"):
    if layer_id == "pafta":
        return refusal("not_a_layer", "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", path), None
    if layer_id not in LAYERS:
        return refusal("layer_not_found", f"“{layer_id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", path), None
    if layer_id == "kilitli":
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", path), None
    if layer_id == "gizli":
        return None, [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": path}]
    return None, []


def revision(inp, conflict):
    rev = inp.get("expectedRevision")
    if rev is None:
        return None
    if not rev.startswith("$") and not (rev.isdigit() and (rev == "0" or not rev.startswith("0"))):
        return refusal("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    if conflict:
        out = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        return out
    return None


def taken_styles(settings, added):
    styles = copy.deepcopy(settings["wallStyles"])
    for s in added:
        if any(w["id"] == s["id"] for w in styles):
            continue
        s = copy.deepcopy(s)
        if any(w["name"].lower() == s["name"].lower() for w in styles):
            s["name"] = s["name"] + " (KentOS)"
        styles.append(s)
    return styles


# ── cad.walls.create ────────────────────────────────────────────────────────────────────────────────────────────


def create(settings, inp, conflict=False):
    """(result, walls to write, styles taken)."""
    if not inp["walls"]:
        return refusal("no_objects", "Yazılacak duvar verilmedi. En az bir duvarın yolunu verin.", "walls"), None, None
    added, out = [], []
    for i, nw in enumerate(inp["walls"]):
        w = build(settings, nw, f"walls[{i}]", added)
        if is_refusal(w):
            return w, None, None
        out.append(w)
    if "lineWeight" in inp and not (math.isfinite(inp["lineWeight"]) and 0 <= inp["lineWeight"] <= 100):
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None, None
    r = revision(inp, conflict)
    if r:
        return r, None, None
    r, warnings = layer_check(inp["layerId"])
    if r:
        return r, None, None
    walls = []
    for nw, w in zip(inp["walls"], out):
        e = {"kind": "wall", "id": 0, "layerId": inp["layerId"]}
        if "color" in inp:
            e["color"] = inp["color"]
        e["attrs"] = nw.get("attrs", {})
        if "label" in nw:
            e["label"] = nw["label"]
        if "lineWeight" in inp:
            e["lineWeight"] = inp["lineWeight"]
        if "lineType" in inp:
            e["lineType"] = inp["lineType"]
        e.update(w)
        walls.append(e)
    return {"status": "completed", "warnings": warnings}, walls, added


# ── The cases' writer ───────────────────────────────────────────────────────────────────────────────────────────

BEFORE = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def with_non_finite(inp, non_finite):
    inp = copy.deepcopy(inp)
    for path, word in non_finite.items():
        value = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}[word]
        parts = path.replace("]", "").replace("[", ".").split(".")
        node = inp
        for key in parts[:-1]:
            node = node[int(key)] if key.isdigit() else node[key]
        last = parts[-1]
        node[int(last) if last.isdigit() else last] = value
    return inp


def settings_after(added):
    return {"wallStyles": taken_styles(SETUP["settings"], added)}


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        assert len(name) > 10
        self.cases.append({"name": name, "steps": steps})


def create_step(op, inp, non_finite=None, conflict=False, note=None):
    result, walls, added = create(SETUP["settings"], with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op}
    if note:
        s["note"] = note
    s["input"] = inp
    if non_finite:
        s["nonFinite"] = non_finite
    if walls is None:
        s["result"] = result
        s["expect"] = dict(BEFORE)
        return s, None
    warnings = result["warnings"]
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": warnings}
        s["expect"] = dict(BEFORE)
        return s, None
    if op == "plan":
        out = {"walls": walls, "revision": "$current"}
        if added:
            out["stylesAdded"] = added
        s["result"] = {"status": "completed", "output": out, "warnings": warnings}
        s["expect"] = dict(BEFORE)
        return s, None
    ids = list(range(FIRST_NEW, FIRST_NEW + len(walls)))
    out = {"uids": ["$any"] * len(walls), "ids": ids}
    if added:
        out["stylesAdded"] = [a["id"] for a in added]
    out["revision"] = "$current"
    s["result"] = {"status": "completed", "output": out, "warnings": warnings}
    entities = {str(i): dict(w, id=i) for i, w in zip(ids, walls)}
    s["expect"] = {"ids": IDS + ids, "entities": entities, "canUndo": True, "dirty": True, "revision": "changed",
                   "settings": settings_after(added)}
    return s, entities


def create_cases():
    c = Cases()
    one = lambda **kw: {"layerId": "duvar", "walls": [kw]}  # noqa: E731
    first, entities = create_step("execute", one(pts=[P(0, 40), P(5, 40)]))
    c.add("düz duvar: genişlik verilmezse 0,2 m, ortası yolda; tek adım “Duvar”, geri alınır ve yinelenir", [
        first,
        {"op": "undo", "returns": "Duvar", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "dirty": True, "revision": "changed"}},
        {"op": "redo", "returns": "Duvar", "expect": {"ids": IDS + [FIRST_NEW], "entities": entities, "canUndo": True, "canRedo": False, "revision": "changed"}},
    ])
    for name, kw in [
        ("genişlik, sol hiza ve öteleme olduğu gibi saklanır", dict(pts=[P(0, 40), P(5, 40), P(5, 44)], width=0.3, justify="left", offset=0.05)),
        ("sağ hizalı duvar: yol sağ yüzüdür", dict(pts=[P(0, 40), P(5, 40)], width=0.25, justify="right")),
        ("yaylı kenar: kenar başına büküm, belge köşe başına saklar (açık yolun sonuncusu 0)", dict(pts=[P(0, 40), P(4, 40), P(7, 43)], bulges=[0.0, 0.25])),
        ("bütün kenarları düz bükümler saklanmaz", dict(pts=[P(0, 40), P(4, 40), P(4, 43)], bulges=[0.0, 0.0])),
        ("kapalı dikdörtgen oda duvarı", dict(pts=[P(0, 40), P(6, 40), P(6, 44), P(0, 44)], closed=True, justify="left")),
        ("iki yarım daireden yuvarlak duvar: kapalı, iki köşe", dict(pts=[P(3, 40), P(-3, 40)], bulges=[1.0, 1.0], closed=True, width=0.3)),
        ("KentOS'un dış duvar stili kimliğiyle: bileşenleri, dolgusu ve 25 cm'si gelir, stil projeye eklenir", dict(pts=[P(0, 40), P(8, 40)], style="kentos-dis-duvar-25")),
        ("KentOS'un stili adıyla (büyük küçük harf ayrımsız) ve verilen genişlikle: kalan bileşen genişliğin kalanını alır", dict(pts=[P(0, 40), P(8, 40)], style="gazbeton 20", width=0.3)),
        ("projenin stili adıyla: proje stili eklenmez; bütün bileşenleri belli olduğundan genişlik toplamdır", dict(pts=[P(0, 40), P(8, 40)], style="Perde 20")),
        ("açık uçlar, yükseklik ve mimari bilgi: kat, evre, durum, disiplin", dict(pts=[P(0, 40), P(5, 40)], caps="open", height=2.8,
                                                                                   arch={"level": "kat-1", "phase": "new", "status": "proposed", "discipline": "architecture"})),
        ("öznitelik ve etiket duvarın kendisinindir", dict(pts=[P(0, 40), P(5, 40)], attrs={"kod": "D-12"}, label="D-12")),
    ]:
        c.add(name, [create_step("execute", one(**kw))[0]])
    c.add("iki duvar tek çağrıda, tek adım; renk, kalınlık ve çizgi tipi ikisinin de", [create_step("execute", {
        "layerId": "duvar", "color": "#7F7F7F", "lineWeight": 0.5, "lineType": "dashed",
        "walls": [{"pts": [P(0, 40), P(5, 40)]}, {"pts": [P(5, 40), P(5, 45)], "style": "kentos-ic-duvar-15"}]})[0]])
    c.add("plan yazmadan duvarı ve alınacak stili gösterir", [create_step("plan", one(pts=[P(0, 40), P(8, 40)], style="Dış duvar 25"))[0]])
    c.add("doğrulama yazmaz", [create_step("validate", one(pts=[P(0, 40), P(8, 40)]))[0]])
    c.add("gizli katmana uyarıyla yazılır", [create_step("execute", {"layerId": "gizli", "walls": [{"pts": [P(0, 40), P(5, 40)]}]})[0]])
    for name, inp, kw in [
        ("duvar verilmezse ret", {"layerId": "duvar", "walls": []}, {}),
        ("sonsuz koordinat reddedilir, yeriyle", one(pts=[P(0, 40), P(5, 40)]), {"non_finite": {"walls[0].pts[1].y": "NaN"}}),
        ("sonsuz genişlik reddedilir", one(pts=[P(0, 40), P(5, 40)], width=0.2), {"non_finite": {"walls[0].width": "Infinity"}}),
        ("aynı yerde iki nokta reddedilir", one(pts=[P(0, 40), P(0, 40), P(5, 40)]), {}),
        ("büküm sayısı kenar sayısından farklıysa ret", one(pts=[P(0, 40), P(5, 40)], bulges=[0.0, 0.0]), {}),
        ("olmayan stil reddedilir; var olanlar söylenir", one(pts=[P(0, 40), P(5, 40)], style="Ahşap 12"), {}),
        ("sıfır genişlik reddedilir", one(pts=[P(0, 40), P(5, 40)], width=0.0), {}),
        ("bileşenleri belli stilde başka genişlik reddedilir", one(pts=[P(0, 40), P(5, 40)], style="Perde 20", width=0.3), {}),
        ("bileşenlere yer bırakmayan genişlik reddedilir", one(pts=[P(0, 40), P(5, 40)], style="kentos-dis-duvar-25", width=0.09), {}),
        ("kapalı iki noktalı düz duvar reddedilir", one(pts=[P(0, 40), P(5, 40)], closed=True), {}),
        ("duvardan dar yay çizilemez", one(pts=[P(0, 40), P(0.1, 40)], bulges=[1.0], width=0.2), {}),
        ("projede olmayan kat reddedilir", one(pts=[P(0, 40), P(5, 40)], arch={"level": "bodrum"}), {}),
        ("kalınlık 100 mm'yi aşarsa ret", {"layerId": "duvar", "lineWeight": 120.0, "walls": [{"pts": [P(0, 40), P(5, 40)]}]}, {}),
        ("kilitli katmana yazılmaz", {"layerId": "kilitli", "walls": [{"pts": [P(0, 40), P(5, 40)]}]}, {}),
        ("katman grubuna yazılmaz", {"layerId": "pafta", "walls": [{"pts": [P(0, 40), P(5, 40)]}]}, {}),
        ("olmayan katman reddedilir", {"layerId": "yok", "walls": [{"pts": [P(0, 40), P(5, 40)]}]}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"layerId": "duvar", "walls": [{"pts": [P(0, 40), P(5, 40)]}], "expectedRevision": "07"}, {}),
    ]:
        c.add(name, [create_step("execute", inp, **kw)[0]])
    stale, _ = create_step("execute", {"layerId": "duvar", "walls": [{"pts": [P(0, 40), P(5, 40)]}], "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [
        {"op": "captureRevision", "as": "prepared"},
        {"op": "execute", "note": "Girdi hazırlandıktan sonra çizim başka bir duvarla değişir.",
         "input": {"layerId": "duvar", "walls": [{"pts": [P(0, 50), P(5, 50)]}]},
         "result": {"status": "completed", "output": {"uids": ["$any"], "ids": [FIRST_NEW], "revision": "$current"}, "warnings": []},
         "expect": {"revision": "changed"}},
        dict(stale, expect={"ids": IDS + [FIRST_NEW], "canUndo": True, "canRedo": False, "revision": "same"}),
    ])
    return c.cases


# ── cad.walls.convert ─────────────────────────────────────────────────────────────────────────────────────────

BY_ID = {e["id"]: e for e in SETUP_ENTITIES}
EPS = 1e-6
UID_TEXT = "0123456789abcdef-"


def uid_of(i):
    return f"$uidOf:{i}"


def captured(i):
    """A source's id as a step before the write took it: the object may be gone after."""
    return f"$uid:o{i}"


def named(inp, nothing):
    """The objects the ids name, each once, in order: (place, id); or the refusal."""
    if not inp["uids"]:
        return refusal("no_entities", f"{nothing} En az bir nesnenin kalıcı kimliğini verin.", "uids")
    out, seen = [], set()
    for at, u in enumerate(inp["uids"]):
        if not u.startswith("$uidOf:"):
            if len(u) != 36 or any(ch not in UID_TEXT for ch in u):
                return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{at}]")
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{at}]")
        i = int(u[len("$uidOf:"):])
        if i in seen:
            continue
        seen.add(i)
        out.append((at, i))
    return out


def path_of(e):
    def per_edge(b, n):
        if b is None or not any(x != 0 for x in b[:n]):
            return None
        return (list(b[:n]) + [0.0] * n)[:n]
    k = e["kind"]
    if k == "line":
        return [e["a"], e["b"]], None, False
    if k == "polyline" and len(e["pts"]) >= 2:
        closed = e.get("closed", False)
        edges = len(e["pts"]) if closed else len(e["pts"]) - 1
        return e["pts"], per_edge(e.get("bulges"), edges), closed
    if k == "polygon" and len(e["pts"]) >= 2:
        return e["pts"], per_edge(e.get("bulges"), len(e["pts"])), True
    if k == "arc":
        sweep = (e["a1"] - e["a0"]) % (2 * math.pi) or 2 * math.pi
        at = lambda t: {"x": e["c"]["x"] + e["r"] * math.cos(t), "y": e["c"]["y"] + e["r"] * math.sin(t)}  # noqa: E731
        return [at(e["a0"]), at(e["a1"])], [math.tan(sweep / 4.0)], False
    if k == "circle":
        return [{"x": e["c"]["x"] + e["r"], "y": e["c"]["y"]}, {"x": e["c"]["x"] - e["r"], "y": e["c"]["y"]}], [1.0, 1.0], True
    return None


def straight(e):
    if e["kind"] == "line":
        return e["a"], e["b"]
    if e["kind"] == "polyline" and not e.get("closed") and len(e["pts"]) == 2 and not any(x != 0 for x in e.get("bulges", [])):
        return e["pts"][0], e["pts"][1]
    return None


def pairs(lines, max_width, angle):
    cands = []
    for x, (i, a, b) in enumerate(lines):
        length = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
        if length <= EPS:
            continue
        u = ((b["x"] - a["x"]) / length, (b["y"] - a["y"]) / length)
        for (j, c, e) in lines[x + 1:]:
            lj = math.hypot(e["x"] - c["x"], e["y"] - c["y"])
            if lj <= EPS:
                continue
            v = ((e["x"] - c["x"]) / lj, (e["y"] - c["y"]) / lj)
            cross = u[0] * v[1] - u[1] * v[0]
            dot = u[0] * v[0] + u[1] * v[1]
            if math.atan2(abs(cross), abs(dot)) * 180.0 / math.pi > angle:
                continue
            mid = ((c["x"] + e["x"]) / 2.0, (c["y"] + e["y"]) / 2.0)
            across = u[0] * (mid[1] - a["y"]) - u[1] * (mid[0] - a["x"])
            d = abs(across)
            if not (d > EPS and d <= max_width):
                continue
            t = lambda p: (p["x"] - a["x"]) * u[0] + (p["y"] - a["y"]) * u[1]  # noqa: E731
            t1, t2 = t(c), t(e)
            lo_j, hi_j = min(t1, t2), max(t1, t2)
            lo, hi = max(lo_j, 0.0), min(hi_j, length)
            if hi - lo <= EPS:
                continue
            cands.append((d, -(hi - lo), i, j, lo, hi, 1.0 if across > 0 else -1.0, lo_j >= lo - EPS and hi_j <= hi + EPS,
                          lo <= EPS and hi >= length - EPS, a, u))
    cands.sort(key=lambda c: (c[0], c[1], c[2], c[3]))
    taken, out = set(), []
    for d, _, i, j, lo, hi, side, j_in, i_whole, a, u in cands:
        if i in taken or j in taken:
            continue
        taken |= {i, j}
        n = (-u[1], u[0])
        half = side * d / 2.0
        at = lambda t: {"x": a["x"] + u[0] * t + n[0] * half, "y": a["y"] + u[1] * t + n[1] * half}  # noqa: E731
        out.append({"i": i, "j": j, "path": [at(lo), at(hi)], "width": d, "covers": (i_whole, j_in)})
    return out


def components_fit(width, comps):
    return not comps or thicknesses_fit(width, comps)[0]


def from_lines(settings, inp, conflict=False):
    """(result, [(place, layer, wall)], removed ids, left ids, styles taken)."""
    found = named(inp, "Duvara çevrilecek çizgi verilmedi.")
    if is_refusal(found):
        return found, None, None, None, None
    locked = [(at, i) for at, i in found if BY_ID[i]["layerId"] == "kilitli"]
    found = [(at, i) for at, i in found if BY_ID[i]["layerId"] != "kilitli"]
    warnings = []
    if not found:
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{locked[0][0]}]"), None, None, None, None
    if locked:
        warnings.append({"code": "layer_locked", "message": f"{len(locked)} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"})
    for key, name in (("width", "Genişlik"), ("maxWidth", "En büyük genişlik"), ("angle", "Açı")):
        if key in inp and not math.isfinite(inp[key]):
            return not_finite(name, key), None, None, None, None
    if "layerId" in inp:
        r, w = layer_check(inp["layerId"])
        if r:
            return r, None, None, None, None
        warnings += w
    layer_of = lambda i: inp.get("layerId", BY_ID[i]["layerId"])  # noqa: E731
    added, walls, removed, left, other, later = [], [], [], [], [], []
    keep = inp.get("keepSources", False)
    style = inp.get("style")
    if inp.get("mode", "each") == "each":
        for k, (at, i) in enumerate(found):
            pth = path_of(BY_ID[i])
            if pth is None:
                other.append((at, i))
                continue
            nw = {"pts": pth[0]}
            if pth[1] is not None:
                nw["bulges"] = pth[1]
            if pth[2]:
                nw["closed"] = True
            for key in ("width", "justify"):
                if key in inp:
                    nw[key] = inp[key]
            if style:
                nw["style"] = style
            w = build(settings, nw, f"uids[{at}]", added)
            if is_refusal(w):
                return w, None, None, None, None
            walls.append((k, layer_of(i), w))
            if not keep:
                removed.append(i)
    else:
        lines = []
        for k, (at, i) in enumerate(found):
            st = straight(BY_ID[i])
            if st is None:
                other.append((at, i))
            else:
                lines.append((k, st[0], st[1]))
        max_width = inp.get("maxWidth", 0.6)
        paired = pairs(lines, max_width, inp.get("angle", 1.0))
        kept = 0
        for p in paired:
            at, i = found[p["i"]]
            nw = {"pts": p["path"], "width": p["width"], "justify": "center"}
            if style:
                s_def, _ = find_style(settings["wallStyles"], style)
                if s_def is None or components_fit(p["width"], s_def["components"]):
                    nw["style"] = style
            w = build(settings, nw, f"uids[{at}]", added)
            if is_refusal(w):
                return w, None, None, None, None
            walls.append((p["i"], layer_of(i), w))
            for k, covers in ((p["i"], p["covers"][0]), (p["j"], p["covers"][1])):
                if keep:
                    continue
                if covers:
                    removed.append(found[k][1])
                else:
                    kept += 1
        if kept:
            later.append({"code": "source_kept", "message": f"{kept} çizginin duvarın örtmediği parçası var; o çizgiler silinmedi.", "path": "uids"})
        used = {x for p in paired for x in (p["i"], p["j"])}
        unpaired = [(found[k][0], found[k][1]) for k, _, _ in lines if k not in used]
        if unpaired:
            if not paired and not other:
                return refusal("wall_unpaired", f"Hiçbir çizginin karşısında {num(max_width)} m içinde paralel ve örtüşen bir çizgi yok. En büyük genişliği artırın ya da çizgileri Tek tek çevirin.", "uids"), None, None, None, None
            later.append({"code": "wall_unpaired", "message": f"{len(unpaired)} çizginin karşısında {num(max_width)} m içinde paralel ve örtüşen bir çizgi yok; duvara çevrilmedi.", "path": f"uids[{unpaired[0][0]}]"})
            left += [i for _, i in unpaired]
    each = inp.get("mode", "each") == "each"
    if not walls:
        at = other[0][0] if other else 0
        msg = ("Verilen nesnelerden duvar çizilmez: çizgi, çoklu çizgi, kapalı alan, yay ya da daire verin." if each else
               "Çiftler yalnız düz çizgileri (çizgi, iki köşeli çoklu çizgi) eşler; verilenlerin hiçbiri düz çizgi değil.")
        return refusal("not_a_wall_path", msg, f"uids[{at}]"), None, None, None, None
    if other:
        why = "çizgi, çoklu çizgi, kapalı alan, yay ya da daire değil" if each else "düz çizgi değil"
        later.append({"code": "not_a_wall_path", "message": f"{len(other)} nesneden duvar çizilmez ({why}); bırakıldı.", "path": f"uids[{other[0][0]}]"})
        left += [i for _, i in other]
    r = revision(inp, conflict)
    if r:
        return r, None, None, None, None
    if "layerId" not in inp:
        seen = []
        for _, layer_id, _ in walls:
            if layer_id not in seen:
                seen.append(layer_id)
                r, w = layer_check(layer_id)
                warnings += w
    walls.sort(key=lambda x: x[0])
    return {"status": "completed", "warnings": warnings + later}, walls, removed, left, added


def wall_entity(id, layer_id, w):
    return {"kind": "wall", "id": id, "layerId": layer_id, "attrs": {}, **w}


def from_step(op, inp, non_finite=None, conflict=False, within=None):
    result, walls, removed, left, added = from_lines(SETUP["settings"], with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if within:
        s["within"] = within
    if walls is None:
        s["result"] = result
        s["expect"] = dict(BEFORE)
        return s
    warnings = result["warnings"]
    ids = list(range(FIRST_NEW, FIRST_NEW + len(walls)))
    entities = [wall_entity(i, layer_id, w) for i, (_, layer_id, w) in zip(ids, walls)]
    if op == "plan":
        out = {"walls": [dict(e, id=0) for e in entities]}
        if removed:
            out["removed"] = [uid_of(i) for i in removed]
        if left:
            out["left"] = [uid_of(i) for i in left]
        if added:
            out["stylesAdded"] = added
        out["revision"] = "$current"
        s["result"] = {"status": "completed", "output": out, "warnings": warnings}
        s["expect"] = dict(BEFORE)
        return s
    out = {"uids": ["$any"] * len(walls)}
    if removed:
        out["removed"] = [captured(i) for i in removed]
    if left:
        out["left"] = [captured(i) for i in left]
    out["revision"] = "$current"
    s["result"] = {"status": "completed", "output": out, "warnings": warnings}
    s["expect"] = {"ids": [i for i in IDS if i not in removed] + ids, "entities": {str(e["id"]): e for e in entities},
                   "canUndo": True, "dirty": True, "revision": "changed", "settings": settings_after(added)}
    s["capture"] = sorted(set(removed) | set(left))
    return s


def from_cases():
    c = Cases()
    first = from_step("execute", {"uids": [uid_of(1)]})
    c.add("tek tek: çizgi boyunca 0,2 m'lik duvar, çizgi silinir; tek adım “Çizgiden duvar”, geri alınınca çizgi döner", [
        *captures(first),
        {"op": "undo", "returns": "Çizgiden duvar", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "dirty": True, "revision": "changed"}},
    ])
    for name, inp, kw in [
        ("kaynak korunursa çizgi kalır", {"uids": [uid_of(1)], "keepSources": True}, {}),
        ("yaylı çoklu çizgi: duvar bükümleriyle", {"uids": [uid_of(6)], "width": 0.25, "justify": "left"}, {}),
        ("kapalı alanın halkası kapalı duvar olur", {"uids": [uid_of(7)], "justify": "right"}, {}),
        ("yay: iki uç ve çeyrek dairenin bükümü tan(π/8)", {"uids": [uid_of(8)]}, {"within": 1e-12}),
        ("daire: iki yarım daireden kapalı duvar", {"uids": [uid_of(9)], "width": 0.3}, {}),
        ("yazı duvar olmaz: bırakılır ve söylenir", {"uids": [uid_of(1), uid_of(10)]}, {}),
        ("stil ve katman verilir: duvarlar duvar katmanında, stilin bileşenleriyle", {"uids": [uid_of(1), uid_of(5)], "style": "Gazbeton 20", "layerId": "duvar"}, {}),
        ("çiftler: paralel iki çizgi aralarındaki uzaklık genişliğinde tek duvar, ikisi de silinir", {"uids": [uid_of(1), uid_of(2)], "mode": "pairs"}, {}),
        ("çiftler: kısa çizgi örtüşen kısımda duvar olur; uzun çizgi kalır ve söylenir", {"uids": [uid_of(3), uid_of(4)], "mode": "pairs"}, {}),
        ("çiftler: eşi olmayan çizgi bırakılır ve söylenir", {"uids": [uid_of(1), uid_of(2), uid_of(5)], "mode": "pairs"}, {}),
        ("çiftler: en yakın önce, eşitlikte ilk çizgi; üçüncü çizgi eşsiz kalır", {"uids": [uid_of(1), uid_of(2), uid_of(17)], "mode": "pairs"}, {}),
        ("çiftler: iki köşeli çoklu çizgi düz çizgidir", {"uids": [uid_of(15), uid_of(16)], "mode": "pairs"}, {"within": 1e-12}),
        ("çiftler: yarım derece eğik çizgi 1° içinde paraleldir", {"uids": [uid_of(18), uid_of(19)], "mode": "pairs"}, {"within": 1e-9}),
        ("çiftler: iki derece eğik çizgi paralel sayılmaz; açı payı 3° verilirse eşleşir", {"uids": [uid_of(20), uid_of(21)], "mode": "pairs", "angle": 3.0}, {"within": 1e-9}),
        ("çiftler: yaylı çoklu çizgi düz çizgi değildir, bırakılır", {"uids": [uid_of(1), uid_of(2), uid_of(6)], "mode": "pairs"}, {}),
        ("çiftler: bileşenleri genişliğe sığmayan stil alınmaz", {"uids": [uid_of(1), uid_of(2)], "mode": "pairs", "style": "Perde 20"}, {}),
        ("çiftler: bileşenleri sığan stil alınır, kaynaklar korunur", {"uids": [uid_of(1), uid_of(2)], "mode": "pairs", "style": "kentos-ic-duvar-15", "keepSources": True}, {}),
        ("kilitli katmandaki kaynak atlanır ve söylenir", {"uids": [uid_of(1), uid_of(11)]}, {}),
        ("plan yazmadan duvarı, silinecek ve kalan kaynakları gösterir", {"uids": [uid_of(1), uid_of(10)]}, {"op": "plan"}),
        ("yalnız kilitli kaynak: ret", {"uids": [uid_of(11)]}, {}),
        ("yalnız yazı: ret", {"uids": [uid_of(10)]}, {}),
        ("çiftler: eşleşen yoksa ret", {"uids": [uid_of(5)], "mode": "pairs"}, {}),
        ("çiftler: en büyük genişlik 0,2 m iken 25 cm aralık eşleşmez: ret", {"uids": [uid_of(1), uid_of(2)], "mode": "pairs", "maxWidth": 0.2}, {}),
        ("çiftler: yalnız yaylı çoklu çizgi: ret", {"uids": [uid_of(6)], "mode": "pairs"}, {}),
        ("kimlik verilmezse ret", {"uids": []}, {}),
        ("geçersiz kimlik reddedilir", {"uids": ["D-1"]}, {}),
        ("çizimde olmayan kimlik reddedilir", {"uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]}, {}),
        ("sonsuz genişlik reddedilir", {"uids": [uid_of(1)], "width": 0.2}, {"non_finite": {"width": "NaN"}}),
        ("olmayan stil reddedilir", {"uids": [uid_of(1)], "style": "Ahşap 12"}, {}),
        ("kilitli katmana yazılmaz", {"uids": [uid_of(1)], "layerId": "kilitli"}, {}),
        ("gizli katmana uyarıyla yazılır", {"uids": [uid_of(1)], "layerId": "gizli"}, {}),
        ("olmayan katman reddedilir", {"uids": [uid_of(1)], "layerId": "yok"}, {}),
        ("katman grubuna yazılmaz", {"uids": [uid_of(1)], "layerId": "pafta"}, {}),
        ("sıfır genişlik reddedilir", {"uids": [uid_of(1)], "width": 0.0}, {}),
        ("yayın yarıçapından geniş duvar çizilemez", {"uids": [uid_of(8)], "width": 7.0}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(1)], "expectedRevision": "07"}, {}),
    ]:
        op = kw.pop("op", "execute")
        c.add(name, captures(from_step(op, inp, **kw)))
    stale = from_step("execute", {"uids": [uid_of(5)], "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [
        {"op": "captureRevision", "as": "prepared"},
        {"op": "captureUid", "id": 1, "as": "o1"},
        {k: v for k, v in from_step("execute", {"uids": [uid_of(1)]}).items() if k != "capture"}
        | {"note": "Girdi hazırlandıktan sonra çizim başka bir dönüştürmeyle değişir.", "expect": {"revision": "changed"}},
        dict(stale, expect={"canUndo": True, "canRedo": False, "revision": "same"}),
    ])
    return c.cases


def captures(step):
    """The steps that take the sources' ids before `step` (an object it removes has no id after it), then `step`."""
    ids = step.pop("capture", [])
    return [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in ids] + [step]


# ── cad.walls.edit ──────────────────────────────────────────────────────────────────────────────────────────────

NULLABLE = ("style", "fill", "height", "level", "phase", "status", "discipline")


def changed(settings, w0, ch, style, at):
    """The wall `w0` (its stored JSON) changed: the new JSON, or the refusal."""
    w = copy.deepcopy(w0)
    if "style" in ch:
        if ch["style"] is None:
            w.pop("style", None)
            w.pop("components", None)
        else:
            w["style"] = style["id"]
            w["components"] = copy.deepcopy(style["components"])
            if style.get("caps") == "open":
                w["caps"] = "open"
            else:
                w.pop("caps", None)
            if "fill" in style:
                w["fill"] = style["fill"]
            else:
                w.pop("fill", None)
            f = fixed_width(style["components"])
            if f is not None:
                w["width"] = f
    for key in ("width", "justify"):
        if key in ch:
            w[key] = ch[key]
    if "offset" in ch:
        if ch["offset"] == 0:
            w.pop("offset", None)
        else:
            w["offset"] = ch["offset"]
    if "caps" in ch:
        if ch["caps"] == "open":
            w["caps"] = "open"
        else:
            w.pop("caps", None)
    for key in ("fill", "height"):
        if key in ch:
            if ch[key] is None:
                w.pop(key, None)
            else:
                w[key] = ch[key]
    arch = dict(w.get("arch", {}))
    for key in ("level", "phase", "status", "discipline"):
        if key in ch:
            if ch[key] is None:
                arch.pop(key, None)
            else:
                arch[key] = ch[key]
    if arch:
        w["arch"] = arch
    else:
        w.pop("arch", None)
    p = wall_problem(w)
    if p:
        return refusal("invalid_wall", p[1], f"operation.{p[0]}")
    if not drawable(w):
        return refusal("wall_collapses", COLLAPSE, at)
    r = level_known(settings, w, "operation.level")
    if r:
        return r
    return w


def edit(settings, inp, conflict=False):
    found = named(inp, "Düzenlenecek duvar verilmedi.")
    if is_refusal(found):
        return found, None, None
    ch = {k: v for k, v in inp["operation"].items() if k != "kind"}
    for at, i in found:
        if BY_ID[i]["kind"] != "wall":
            return refusal("not_a_wall", "Nesne duvar değil; Duvarı düzenle yalnız duvarları değiştirir.", f"uids[{at}]"), None, None
    if not ch:
        return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi: genişlik, hiza, öteleme, stil, uçlar, dolgu, yükseklik, kat, evre, durum ya da disiplin verin.", "operation"), None, None
    for key, name in (("width", "Genişlik"), ("offset", "Öteleme"), ("height", "Yükseklik")):
        if ch.get(key) is not None and not math.isfinite(ch[key]):
            return not_finite(name, f"operation.{key}"), None, None
    added, style = [], None
    if ch.get("style") is not None:
        style, builtin = find_style(settings["wallStyles"], ch["style"])
        if style is None:
            return no_style(settings["wallStyles"], ch["style"], "operation.style"), None, None
        if builtin:
            added.append(style)
    out = []
    for at, i in found:
        w0 = {k: v for k, v in BY_ID[i].items() if k not in ("kind", "id", "layerId", "attrs")}
        w = changed(settings, w0, ch, style, f"uids[{at}]")
        if is_refusal(w):
            return w, None, None
        out.append((i, {"kind": "wall", "id": i, "layerId": BY_ID[i]["layerId"], "attrs": {}, **w}))
    r = revision(inp, conflict)
    if r:
        return r, None, None
    for at, i in found:
        if BY_ID[i]["layerId"] == "kilitli":
            return refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki duvar düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None, None
    return {"status": "completed", "warnings": []}, out, added


def edit_step(op, inp, non_finite=None, conflict=False):
    result, walls, added = edit(SETUP["settings"], with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if walls is None:
        s["result"] = result
        s["expect"] = dict(BEFORE)
        return s
    out = {"walls": [e for _, e in walls]}
    if added:
        out["stylesAdded"] = [a["id"] for a in added]
    out["revision"] = "$current"
    s["result"] = {"status": "completed", "output": out, "warnings": []}
    if op in ("plan", "validate"):
        if op == "validate":
            s["result"]["output"] = None
        s["expect"] = dict(BEFORE)
        return s
    s["expect"] = {"ids": IDS, "entities": {str(i): e for i, e in walls}, "canUndo": True, "dirty": True, "revision": "changed",
                   "settings": settings_after(added)}
    return s


def edit_cases():
    c = Cases()
    op = lambda **kw: {"kind": "set", **kw}  # noqa: E731
    first = edit_step("execute", {"uids": [uid_of(12)], "operation": op(width=0.3)})
    c.add("genişlik değişir; tek adım “Duvarı düzenle”, geri alınınca eski duvar döner", [
        first,
        {"op": "undo", "returns": "Duvarı düzenle", "expect": {"ids": IDS, "entities": {"12": BY_ID[12]}, "canUndo": False, "canRedo": True, "dirty": True, "revision": "changed"}},
    ])
    moved = edit_step("execute", {"uids": [uid_of(12)], "operation": op(offset=0.05)})
    back = copy.deepcopy(moved)
    back["input"] = {"uids": [uid_of(12)], "operation": op(offset=0.0)}
    back["result"]["output"]["walls"] = [BY_ID[12]]
    back["expect"] = {"ids": IDS, "entities": {"12": BY_ID[12]}, "canUndo": True, "revision": "changed"}
    c.add("öteleme 0 olunca kalkar: iki adım, ikincisi duvarı ilk hâline döndürür", [moved, back])
    for name, inp, kw in [
        ("sağ hiza ve öteleme", {"uids": [uid_of(12)], "operation": op(justify="right", offset=0.05)}, {}),

        ("KentOS'un stili: bileşenleri ve dolgusu gelir, genişlik kalır, stil projeye eklenir", {"uids": [uid_of(12)], "operation": op(style="kentos-dis-duvar-25", width=0.3)}, {}),
        ("projenin stili: bileşenleri genişliği belirler", {"uids": [uid_of(12)], "operation": op(style="Perde 20")}, {}),
        ("stil kaldırılınca tek gövde kalır, dolgu kalır", {"uids": [uid_of(13)], "operation": op(style=None)}, {}),
        ("uçlar açılır", {"uids": [uid_of(12)], "operation": op(caps="open")}, {}),
        ("dolgu kaldırılır", {"uids": [uid_of(13)], "operation": op(fill=None)}, {}),
        ("yükseklik verilir", {"uids": [uid_of(12)], "operation": op(height=2.7)}, {}),
        ("kat değişir, evre, durum ve disiplin verilir", {"uids": [uid_of(13)], "operation": op(level="kat-1", phase="demolish", status="built", discipline="structure")}, {}),
        ("kat kaldırılınca mimari bilgi de kalkar", {"uids": [uid_of(13)], "operation": op(level=None)}, {}),
        ("iki duvar birlikte: ikisi de değişir", {"uids": [uid_of(12), uid_of(22)], "operation": op(width=0.15, phase="new")}, {}),
        ("plan yazmadan değişmiş duvarları gösterir", {"uids": [uid_of(12)], "operation": op(width=0.3)}, {"op": "plan"}),
        ("doğrulama yazmaz", {"uids": [uid_of(12)], "operation": op(width=0.3)}, {"op": "validate"}),
        ("kimlik verilmezse ret", {"uids": [], "operation": op(width=0.3)}, {}),
        ("geçersiz kimlik reddedilir", {"uids": ["D-1"], "operation": op(width=0.3)}, {}),
        ("çizimde olmayan kimlik reddedilir", {"uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"], "operation": op(width=0.3)}, {}),
        ("duvar olmayan nesne reddedilir", {"uids": [uid_of(1)], "operation": op(width=0.3)}, {}),
        ("değişecek alan yoksa ret", {"uids": [uid_of(12)], "operation": op()}, {}),
        ("sonsuz genişlik reddedilir", {"uids": [uid_of(12)], "operation": op(width=0.3)}, {"non_finite": {"operation.width": "Infinity"}}),
        ("olmayan stil reddedilir", {"uids": [uid_of(12)], "operation": op(style="Ahşap 12")}, {}),
        ("sıfır genişlik reddedilir", {"uids": [uid_of(12)], "operation": op(width=0.0)}, {}),
        ("bileşenleri belli stilde başka genişlik reddedilir", {"uids": [uid_of(13)], "operation": op(width=0.3)}, {}),
        ("yaylı duvar yayından geniş olamaz: çizilemez", {"uids": [uid_of(22)], "operation": op(width=5.0)}, {}),
        ("projede olmayan kat reddedilir", {"uids": [uid_of(12)], "operation": op(level="bodrum")}, {}),
        ("kilitli katmandaki duvar bütün düzenlemeyi reddeder", {"uids": [uid_of(12), uid_of(14)], "operation": op(width=0.3)}, {}),
        ("geçersiz beklenen sürüm reddedilir", {"uids": [uid_of(12)], "operation": op(width=0.3), "expectedRevision": "07"}, {}),
    ]:
        o = kw.pop("op", "execute")
        c.add(name, [edit_step(o, inp, **kw)])
    stale = edit_step("execute", {"uids": [uid_of(12)], "operation": op(width=0.4), "expectedRevision": "$prepared"}, conflict=True)
    c.add("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", [
        {"op": "captureRevision", "as": "prepared"},
        dict(edit_step("execute", {"uids": [uid_of(12)], "operation": op(width=0.3)}), note="Girdi hazırlandıktan sonra duvar başka bir düzenlemeyle değişir."),
        dict(stale, expect={"canUndo": True, "canRedo": False, "revision": "same"}),
    ])
    return c.cases


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note,
            "setup": SETUP, "cases": cases}


def files():
    return {
        "cad.walls.create.json": document(
            "cad.walls.create",
            "Duvar oluştur: yol, genişlik, hiza, öteleme, stil, uçlar ve mimari bilgi; doğrulama, plan, yazma, geri alma, retler",
            "ADR 0339 §5. Beklenenler scripts/fixtures/wall_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "yüzlerin çizilebilirliği scripts/fixtures/wall_cases.py'nin ötelemesinden. Kurulumdaki çizimde 22 nesne var; yeni nesne 23'ten başlar.",
            create_cases()),
        "cad.walls.convert.json": document(
            "cad.walls.convert",
            "Çizgiden duvar: tek tek ve çiftler; kaynakların silinmesi ve korunması, bırakılanlar, retler",
            "ADR 0339 §5. Beklenenler scripts/fixtures/wall_command_cases.py'nin ADR'den yazılmış kurallarından (eşlemenin sırası, örtüşme, "
            "duvarın yeri), KentOS kodu olmadan. Kurulumdaki çizimde 22 nesne var; yeni nesne 23'ten başlar.",
            from_cases()),
        "cad.walls.edit.json": document(
            "cad.walls.edit",
            "Duvarı düzenle: genişlik, hiza, öteleme, stil, uçlar, dolgu, yükseklik ve mimari bilgi; hepsi ya da hiçbiri",
            "ADR 0339 §5. Beklenenler scripts/fixtures/wall_command_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan.",
            edit_cases()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
