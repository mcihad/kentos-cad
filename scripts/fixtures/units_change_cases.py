"""Writes fixtures/commands/v1/cad.units.change.json: the shared cases of `cad.units.change` v1 (docs/adr/0256 §1).

The expectations are built here from the ADR's rule, without KentOS code:

- `keep` keeps the sizes: only the drawing unit changes, nothing of the drawing moves;
- `scale` keeps the numbers: every length the drawing measures in its unit is multiplied by the units' ratio, one
  whole factor (×1000 from mm to m, ÷1000 from m to mm, ÷10 from cm to mm …) applied as one float64 operation, as a
  typed value is turned into metres: coordinates, lengths, radii, elevations (z), a text's, a dimension's, a
  leader's and a table's height and sizes, a paragraph's box, a curved text's curve, a hatch's line spacing and its
  pattern's metres per unit, a picture's and a raster's frame, block definitions (base point, objects, attribute
  definitions), the home view and the anchor; angles, ratios (bulges, a dimension's and a leader's sizes, which are
  times their height), an insert's own scale, paper sizes, a route's measures (M) and a point cloud's points stay;
- the unit and the scaled drawing are one undo step “Birimi değiştir”; metres are written as no unit;
- the refusals and their order are the contract's (crates/shared/contracts/src/cad_units.rs).

    python3 scripts/fixtures/units_change_cases.py           # writes the file
    python3 scripts/fixtures/units_change_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.units.change.json"

STEP = "Birimi değiştir"
PER_METRE = {"mm": 1000.0, "cm": 100.0, "m": 1.0}
# A unit's metres as a ratio of whole numbers (docs/adr/0262 §1): an inch is 254/10000 m, a foot 3048/10000.
METRES = {"mm": (1, 1000), "cm": (1, 100), "m": (1, 1), "in": (254, 10000), "ft": (3048, 10000)}

INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
CLOUDS = "Nokta bulutlarının noktaları dosyalarındadır; 1 bulut ölçeklenmedi."
TOLERANCES = "Projenin toleransları (topoloji, ağlar) değişmedi; yeni boylara göre gözden geçirin."


class Factor:
    """The units' ratio as the ADR writes it: one whole number, multiplied or divided by (`mul` or `div` is 1)."""

    def __init__(self, src, dst):
        # The metres a number stood for in `src` become those it stands for in `dst`: the ratio in lowest terms.
        (sn, sd), (dn, dd) = METRES[src], METRES[dst]
        mul, div = dn * sd, dd * sn
        g = math.gcd(mul, div)
        self.mul, self.div = float(mul // g), float(div // g)

    def __call__(self, v):
        if self.mul == 1.0:
            return v / self.div
        if self.div == 1.0:
            return v * self.mul
        return v * self.mul / self.div

    @property
    def value(self):
        return self.mul / self.div


STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE), "children": []}


def p(x, y):
    return {"x": x, "y": y}


def base(id_, layer="cizim", **extra):
    return {"id": id_, "layerId": layer, "attrs": {}, **extra}


# A drawing in metres whose numbers a millimetre project reads ×1000: one object of each kind, every length field
# given a value that is not a whole number of metres, every field that stays given one too.
ENTITIES = [
    {"kind": "point", **base(1), "p": p(1.25, 0.5), "z": 0.75, "parts": [{"p": p(2.5, 0.5), "z": 0.8}]},
    {"kind": "line", **base(2, lineWeight=0.35), "a": p(0.0, 0.0), "b": p(4.0, 0.0), "za": 0.1, "zb": 0.2, "ma": 0.0, "mb": 4.0},
    {"kind": "polyline", **base(3), "pts": [p(0.0, 1.0), p(2.0, 1.5), p(3.25, 1.0)], "bulges": [0.5, 0.0], "zs": [0.1, None, 0.3], "ms": [0.0, None, 7.5]},
    {"kind": "polygon", **base(4), "pts": [p(0.0, 2.0), p(4.0, 2.0), p(4.0, 5.0), p(0.0, 5.0)],
     "holes": [{"pts": [p(1.0, 3.0), p(2.0, 3.0), p(2.0, 4.0)], "zs": [0.5, 0.5, None]}],
     "parts": [{"pts": [p(5.0, 2.0), p(6.0, 2.0), p(6.0, 3.0)], "holes": [{"pts": [p(5.2, 2.2), p(5.6, 2.2), p(5.6, 2.6)]}]}]},
    {"kind": "circle", **base(5), "c": p(6.0, 1.0), "r": 0.5},
    {"kind": "arc", **base(6), "c": p(6.0, 2.0), "r": 0.75, "a0": 0.5, "a1": 2.0},
    {"kind": "ellipse", **base(7), "c": p(8.0, 1.0), "major": p(1.0, 0.5), "ratio": 0.5, "t0": 0.0, "t1": 3.0},
    {"kind": "spline", **base(8), "pts": [p(8.0, 3.0), p(9.0, 3.5), p(10.0, 3.0)], "closed": False},
    {"kind": "xline", **base(9), "p": p(0.0, 6.0), "dir": p(0.6, 0.8)},
    {"kind": "text", **base(10, "yazi"), "p": p(1.0, 6.0), "text": "Salon", "height": 0.25, "rotation": 30.0, "widthFactor": 0.8,
     "boxWidth": 2.5, "lineSpacing": 1.2},
    {"kind": "text", **base(11, "yazi"), "p": p(1.0, 7.0), "text": "Bahçe", "height": 0.2, "rotation": 0.0,
     "path": {"pts": [p(1.0, 0.25), p(2.5, 0.0)], "bulges": [0.25, 0.0]}},
    {"kind": "dimension", **base(12, "olcu"), "a": p(0.0, 0.0), "b": p(4.0, 0.0), "offset": 0.5, "height": 0.18, "arrowSize": 1.2, "extOffset": 0.4},
    {"kind": "dimension", **base(13, "olcu"), "a": p(0.0, 0.0), "b": p(3.0, 4.0), "offset": -0.75, "height": 0.18, "style": "slope", "za": 1.0, "zb": 1.5},
    {"kind": "hatch", **base(14, "tarama"), "ring": [p(0.0, 8.0), p(2.0, 8.0), p(2.0, 9.0)], "holes": [[p(0.5, 8.2), p(1.0, 8.2), p(1.0, 8.5)]],
     "pattern": {"type": "lines", "angle": 45.0, "spacing": 0.2}},
    {"kind": "hatch", **base(15, "tarama"), "ring": [p(3.0, 8.0), p(5.0, 8.0), p(5.0, 9.0)],
     "pattern": {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": "ANSI31", "scale": 0.05,
                 "lines": [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 3.175], "dashes": [0.5, -0.25]}]}},
    {"kind": "hatch", **base(16, "tarama"), "ring": [p(6.0, 8.0), p(7.0, 8.0), p(7.0, 9.0)], "pattern": {"type": "solid", "angle": 0.0, "spacing": 1.0}},
    {"kind": "insert", **base(17), "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5101", "p": p(10.0, 1.0), "scale": 2.0, "rotation": 0.3},
    {"kind": "leader", **base(18, "yazi"), "pts": [p(9.0, 5.0), p(10.0, 6.0)], "text": "Not", "height": 0.2, "rotation": 0.0, "arrowSize": 1.5},
    {"kind": "table", **base(19, "yazi"), "p": p(11.0, 9.0), "rotation": 0.0, "height": 0.2, "rows": [0.5, 0.5], "columns": [1.0, 2.25],
     "cells": [["A", "B"], ["1", "2"]], "frame": 0.05},
    {"kind": "image", **base(20), "p": p(12.0, 0.0), "width": 2.0, "height": 1.5, "rotation": 0.1, "file": "foto.png",
     "clip": [p(0.0, 0.0), p(1.0, 0.0), p(0.5, 1.0)], "opacity": 0.5},
    {"kind": "raster", **base(21), "affine": [20.0, 0.05, 0.0, 10.0, 0.0, -0.05], "width": 200, "height": 100, "bands": 1, "sample": "u8",
     "file": "tarama.tif", "srid": 0, "style": {"render": "gray", "bands": [1]}},
    {"kind": "pointcloud", **base(22), "sources": [{"file": "bulut.laz", "format": "laz", "count": 1000, "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0]}],
     "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0], "count": 1000, "srid": 0, "style": {"render": "rgb", "size": 2}},
]
IDS = [e["id"] for e in ENTITIES]

BLOCKS = [{
    "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5101",
    "name": "Kapı",
    "base": p(0.1, 0.2),
    "entities": [
        {"kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": p(0.0, 0.0), "b": p(0.9, 0.0)},
        {"kind": "arc", "id": 2, "layerId": "cizim", "attrs": {}, "c": p(0.0, 0.0), "r": 0.9, "a0": 0.0, "a1": 1.5707963267948966},
    ],
    "attributes": [{"tag": "NO", "p": p(0.45, 0.3), "height": 0.15, "rotation": 0.0}],
}]

HOME = {"minX": 0.0, "minY": 0.0, "maxX": 14.5, "maxY": 9.5}
ORIGIN = p(5.0, 3.25)
SETTINGS = {"srid": 0, "lengthDecimals": 1, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50, "workspace": "cad",
            "drawingUnit": "mm", "topology": {"tolerance": 0.001, "rules": [], "exceptions": []}}


def setup(entities=None, unit="mm", locked=(), **over):
    s = {**SETTINGS, **over}
    if unit == "m":
        s.pop("drawingUnit")
    else:
        s["drawingUnit"] = unit
    for k in [k for k, v in s.items() if v is None]:
        s.pop(k)
    layers = [node(i, n, locked=i in locked) for i, n in (("cizim", "Çizim"), ("yazi", "Yazı"), ("olcu", "Ölçü"), ("tarama", "Tarama"))]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Birimler",
        "settings": s,
        "origin": dict(ORIGIN),
        "homeView": dict(HOME),
        "layers": layers,
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES if entities is None else entities),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── The rule: what a scale multiplies ─────────────────────────────────────────────────────────────────────────────

def scale_point(v, f):
    return {"x": f(v["x"]), "y": f(v["y"])}


def scale_zs(zs, f):
    return None if zs is None else [None if z is None else f(z) for z in zs]


def scale_ring(r, f):
    out = dict(r, pts=[scale_point(q, f) for q in r["pts"]])
    if "zs" in r:
        out["zs"] = scale_zs(r["zs"], f)
    return out


def scale_entity(e, f):
    e = copy.deepcopy(e)
    k = e["kind"]
    if k == "pointcloud":
        return e  # its points are in its files
    if k == "raster":
        e["affine"] = [f(v) for v in e["affine"]]  # its corner and its pixels' steps; its width and height are pixels
        return e
    for key in ("p", "a", "b", "c", "major"):  # an xline's direction is a unit vector: it stays
        if key in e:
            e[key] = scale_point(e[key], f)
    for key in ("z", "za", "zb", "r", "offset", "height", "width", "boxWidth", "frame"):
        if key in e:
            e[key] = f(e[key])
    if k == "point":
        for part in e.get("parts", []):
            part["p"] = scale_point(part["p"], f)
            if "z" in part:
                part["z"] = f(part["z"])
    if k in ("polyline", "polygon", "spline", "leader"):
        e["pts"] = [scale_point(q, f) for q in e["pts"]]
        if "zs" in e:
            e["zs"] = scale_zs(e["zs"], f)
        e["holes"] = [scale_ring(h, f) for h in e["holes"]] if "holes" in e else None
        if e["holes"] is None:
            e.pop("holes")
        if "parts" in e:
            e["parts"] = [dict(scale_ring(part, f), **({"holes": [scale_ring(h, f) for h in part["holes"]]} if "holes" in part else {}))
                          for part in e["parts"]]
    if k == "text" and "path" in e:
        e["path"]["pts"] = [scale_point(q, f) for q in e["path"]["pts"]]
    if k == "hatch":
        e["ring"] = [scale_point(q, f) for q in e["ring"]]
        if "holes" in e:
            e["holes"] = [[scale_point(q, f) for q in h] for h in e["holes"]]
        pat = e["pattern"]
        if pat["type"] in ("lines", "cross"):
            pat["spacing"] = f(pat["spacing"])
        if "scale" in pat:
            pat["scale"] = f(pat["scale"])
    if k == "table":
        e["rows"] = [f(v) for v in e["rows"]]
        e["columns"] = [f(v) for v in e["columns"]]
    return e


def scale_block(b, f):
    b = copy.deepcopy(b)
    b["base"] = scale_point(b["base"], f)
    b["entities"] = [scale_entity(e, f) for e in b["entities"]]
    b["attributes"] = [dict(a, p=scale_point(a["p"], f), height=f(a["height"])) for a in b["attributes"]]
    return b


def scale_bounds(bx, f):
    return {k: f(v) for k, v in bx.items()}


def blocks_without_ids(blocks):
    return [{k: v for k, v in b.items() if k != "id"} for b in blocks]


# ── Cases ─────────────────────────────────────────────────────────────────────────────────────────────────────────

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


def output(src, dst, geometry, factor, objects, blocks, revision="$current"):
    return {"from": src, "to": dst, "geometry": geometry, "factor": factor, "objects": objects, "blocks": blocks, "revision": revision}


def done(out, warnings=()):
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def warning(code, message):
    return {"code": code, "message": message}


def failed(code, message, path=None, uids=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    if uids:
        error["uids"] = uids
    return {"status": "failed", "error": error}


def unit_setting(u):
    return None if u == "m" else u


def scaled_expect(entities, src, dst):
    f = Factor(src, dst)
    return {
        "ids": [e["id"] for e in entities],
        "entities": {str(e["id"]): scale_entity(e, f) for e in entities},
        "blocks": blocks_without_ids([scale_block(b, f) for b in BLOCKS]),
        "homeView": scale_bounds(HOME, f),
        "origin": scale_point(ORIGIN, f),
        "settings": {"drawingUnit": unit_setting(dst)},
    }


def unchanged_expect(entities, unit):
    return {
        "ids": [e["id"] for e in entities],
        "entities": {str(e["id"]): copy.deepcopy(e) for e in entities},
        "blocks": blocks_without_ids(BLOCKS),
        "homeView": dict(HOME),
        "origin": dict(ORIGIN),
        "settings": {"drawingUnit": unit_setting(unit)},
    }


def scaled_count(entities):
    return sum(1 for e in entities if e["kind"] != "pointcloud")


def warnings_for(entities):
    out = []
    if any(e["kind"] == "pointcloud" for e in entities):
        out.append(warning("point_clouds_kept", CLOUDS))
    out.append(warning("tolerances_kept", TOLERANCES))
    return out


NO_CLOUD = [e for e in ENTITIES if e["kind"] != "pointcloud"]


def refused(name, input_, code, message, path=None, setup_doc=None, uids=None):
    c = {"name": name, "steps": [step("execute", input_, failed(code, message, path, uids),
                                      {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"})]}
    if setup_doc is not None:
        c["setup"] = setup_doc
    return c


def cases():
    out = []
    n, nb = scaled_count(ENTITIES), len(BLOCKS)

    out.append({"name": "mm'den m'ye, boylar korunur: yalnız birim değişir; nesneler, bloklar, ana görünüm aynı",
                "steps": [step("execute", {"from": "mm", "to": "m", "geometry": "keep"}, done(output("mm", "m", "keep", 1.0, 0, 0)),
                               {**unchanged_expect(ENTITIES, "m"), "canUndo": True, "dirty": True, "revision": "changed"}),
                          step("undo", returns=STEP, expect={**unchanged_expect(ENTITIES, "mm"), "canUndo": False})]})

    f = Factor("mm", "m")
    out.append({"name": "mm'den m'ye, sayılar korunur: her uzunluk ×1000, açılar, oranlar ve ölçüler (M) kalır; geri al ikisini birlikte verir",
                "steps": [step("execute", {"from": "mm", "to": "m", "geometry": "scale"}, done(output("mm", "m", "scale", f.value, n, nb), warnings_for(ENTITIES)),
                               {**scaled_expect(ENTITIES, "mm", "m"), "canUndo": True, "dirty": True, "revision": "changed"}),
                          step("undo", returns=STEP, expect={**unchanged_expect(ENTITIES, "mm"), "canUndo": False, "canRedo": True}),
                          step("redo", returns=STEP, expect=scaled_expect(ENTITIES, "mm", "m"))]})

    for src, dst in (("mm", "cm"), ("cm", "mm"), ("cm", "m"), ("m", "cm"), ("m", "mm")):
        f = Factor(src, dst)
        out.append({"name": f"{src}'den {dst}'ye, sayılar korunur: çarpan {'×' if f.div == 1 else '÷'}{int(f.mul if f.div == 1 else f.div)}",
                    "setup": setup(unit=src),
                    "steps": [step("execute", {"from": src, "to": dst, "geometry": "scale"}, done(output(src, dst, "scale", f.value, n, nb), warnings_for(ENTITIES)),
                                   {**scaled_expect(ENTITIES, src, dst), "canUndo": True})]})

    # Feet and inches (docs/adr/0262 §1): the ratio of whole numbers, applied as one product and one division.
    for src, dst in (("mm", "in"), ("in", "ft"), ("ft", "m")):
        f = Factor(src, dst)
        out.append({"name": f"{src}'den {dst}'ye, sayılar korunur: çarpan {int(f.mul)}/{int(f.div)} (ADR 0262)",
                    "setup": setup(unit=src),
                    "steps": [step("execute", {"from": src, "to": dst, "geometry": "scale"}, done(output(src, dst, "scale", f.value, n, nb), warnings_for(ENTITIES)),
                                   {**scaled_expect(ENTITIES, src, dst), "canUndo": True})]})
    out.append({"name": "inç'ten ayağa, boylar korunur (ADR 0262)",
                "setup": setup(unit="in"),
                "steps": [step("execute", {"from": "in", "to": "ft", "geometry": "keep"}, done(output("in", "ft", "keep", 1.0, 0, 0)),
                               {**unchanged_expect(ENTITIES, "ft"), "canUndo": True})]})

    out.append({"name": "m'den mm'ye, boylar korunur",
                "setup": setup(unit="m"),
                "steps": [step("execute", {"from": "m", "to": "mm", "geometry": "keep"}, done(output("m", "mm", "keep", 1.0, 0, 0)),
                               {**unchanged_expect(ENTITIES, "mm"), "canUndo": True})]})

    # Metre sanılan milimetrelik çizim: 4000 m'lik duvar 4000 mm okunur, 4 m'dir.
    wall = [{"kind": "line", **base(1), "a": p(0.0, 0.0), "b": p(4000.0, 0.0)},
            {"kind": "polygon", **base(2), "pts": [p(0.0, 0.0), p(4000.0, 0.0), p(4000.0, 3000.0), p(0.0, 3000.0)]}]
    f = Factor("m", "mm")
    out.append({"name": "Metre sanılan milimetrelik oda: 4000 × 3000 sayıları kalır, oda 4 × 3 m olur",
                "setup": setup(wall, unit="m", topology=None),
                "steps": [step("execute", {"from": "m", "to": "mm", "geometry": "scale"}, done(output("m", "mm", "scale", f.value, 2, nb)),
                               {"entities": {"1": {**wall[0], "b": p(4.0, 0.0)}, "2": {**wall[1], "pts": [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)]}},
                                "settings": {"drawingUnit": "mm"}})]})

    out.append({"name": "Boş çizim ölçeklenince ana görünüm ve çapa ölçeklenir; nesne yok",
                "setup": setup([], topology=None),
                "steps": [step("execute", {"from": "mm", "to": "m", "geometry": "scale"}, done(output("mm", "m", "scale", 1000.0, 0, nb)),
                               {"ids": [], "homeView": scale_bounds(HOME, Factor("mm", "m")), "origin": scale_point(ORIGIN, Factor("mm", "m")),
                                "blocks": blocks_without_ids([scale_block(b, Factor("mm", "m")) for b in BLOCKS])})]})

    out.append({"name": "Bulutsuz, toleranssız çizim: uyarı yok",
                "setup": setup(NO_CLOUD, topology=None),
                "steps": [step("execute", {"from": "mm", "to": "cm", "geometry": "scale"}, done(output("mm", "cm", "scale", 10.0, len(NO_CLOUD), nb)),
                               {"entities": {str(e["id"]): scale_entity(e, Factor("mm", "cm")) for e in NO_CLOUD}})]})

    out.append({"name": "Plan: ne olacağını söyler, yazmaz",
                "steps": [step("plan", {"from": "mm", "to": "m", "geometry": "scale"}, done(output("mm", "m", "scale", 1000.0, n, nb), warnings_for(ENTITIES)),
                               {**unchanged_expect(ENTITIES, "mm"), "canUndo": False, "dirty": False, "revision": "same"})]})
    out.append({"name": "Plan, boylar korunurken: çarpan 1",
                "steps": [step("plan", {"from": "mm", "to": "cm", "geometry": "keep"}, done(output("mm", "cm", "keep", 1.0, 0, 0)),
                               {"settings": {"drawingUnit": "mm"}, "revision": "same"})]})
    out.append({"name": "Doğrula: yazmaz",
                "steps": [step("validate", {"from": "mm", "to": "m", "geometry": "scale"}, {"status": "completed", "output": None, "warnings": []},
                               {"settings": {"drawingUnit": "mm"}, "revision": "same", "canUndo": False})]})

    locked_layer = setup(locked=("tarama",))
    out.append({"name": "Kilitli katmanda nesne varken boylar korunabilir",
                "setup": locked_layer,
                "steps": [step("execute", {"from": "mm", "to": "m", "geometry": "keep"}, done(output("mm", "m", "keep", 1.0, 0, 0)),
                               {"settings": {"drawingUnit": None}, "canUndo": True})]})
    out.append(refused("kilitli katmanda nesne varken sayılar korunamaz", {"from": "mm", "to": "m", "geometry": "scale"}, "layer_locked",
                       "Kilitli katmanda nesne var (Tarama: 3 nesne); ölçeklemek bütün çizimi değiştirir, hiçbir şey yazılmadı. Katmanın kilidini açın ya da boyları koruyun (geometry: keep).",
                       "geometry", locked_layer, uids=["$uidOf:14", "$uidOf:15", "$uidOf:16"]))
    out.append(refused("aynı birim", {"from": "mm", "to": "mm", "geometry": "scale"}, "same_unit",
                       "Çizimin birimi zaten mm; değişecek bir şey yok.", "to"))
    out.append(refused("çizimin birimi başka", {"from": "cm", "to": "m", "geometry": "keep"}, "unit_mismatch",
                       "Çizimin birimi mm, cm değil; from çizimin şimdiki birimi olmalı. Komutu çizimin şimdiki birimiyle yeniden hazırlayın.", "from"))
    out.append(refused("metre çizimde from mm", {"from": "mm", "to": "cm", "geometry": "scale"}, "unit_mismatch",
                       "Çizimin birimi m, mm değil; from çizimin şimdiki birimi olmalı. Komutu çizimin şimdiki birimiyle yeniden hazırlayın.", "from",
                       setup(unit="m")))
    out.append(refused("koordinat sistemli proje", {"from": "m", "to": "mm", "geometry": "keep"}, "unit_needs_local",
                       "Koordinat sistemi olan projenin birimi metredir; mm ya da cm yalnız sistemsiz yerel projede seçilir.", "to",
                       setup(unit="m", srid=5256, workspace="gis")))
    out.append(refused("aynı birim, sürümden önce", {"from": "mm", "to": "mm", "geometry": "keep", "expectedRevision": "on iki"}, "same_unit",
                       "Çizimin birimi zaten mm; değişecek bir şey yok.", "to"))
    out.append(refused("geçersiz beklenen sürüm", {"from": "mm", "to": "m", "geometry": "keep", "expectedRevision": "on iki"}, "invalid_revision",
                       INVALID_REVISION, "expectedRevision"))
    out.append({"name": "sürüm çakışması: hiçbir şey yazılmaz",
                "steps": [step("captureRevision", **{"as": "once"}),
                          step("execute", {"from": "mm", "to": "cm", "geometry": "keep"}, done(output("mm", "cm", "keep", 1.0, 0, 0)), {"revision": "changed"}),
                          step("execute", {"from": "cm", "to": "m", "geometry": "scale", "expectedRevision": "$once"},
                               {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                               {"revision": "same", "settings": {"drawingUnit": "cm"}, "entities": {"2": ENTITIES[1]}})]})
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.units.change",
        "commandVersion": 1,
        "title": "Birimi değiştir",
        "note": "Written by scripts/fixtures/units_change_cases.py from docs/adr/0256 §1 and the contract's rules; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    doc = build()
    text_ = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.units.change durumları güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
