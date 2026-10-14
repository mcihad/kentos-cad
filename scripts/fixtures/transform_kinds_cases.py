"""Every object kind through Taşı, Döndür, Ölçekle and Aynala: shared cases of cad.entities.transform (docs/adr/0291).

    python3 scripts/fixtures/transform_kinds_cases.py           # writes the file
    python3 scripts/fixtures/transform_kinds_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.transform.kinds.json beside transform_command_cases.py's file: a drawing with
one object of every kind (and the variants a kind keeps: a point's extra points, a polyline's bulges, measures,
elevations and widths, a closed polyline, a multi-part polyline, an area's holes and parts, a NURBS curve with its fit
points and tangents, a line and a ray without end, a multi-line text, a text along a curve, a dimension of each
sideways rule, the three hatch fills, a block insert with attributes, a leader, a table, an image with a clip, a
raster, a point cloud), every one moved, turned, scaled and mirrored (plan: the objects as they would be, nothing
written), copied (the copy count). An associative dimension following its object is docs/adr/0249's cases' (its ties
are the document's, before the record).

The geometry is computed independently from the transforms' definitions (affine_reference.py, beside this file, and the
rules below, written from the ADRs: 0037, 0139, 0142, 0143, 0144 §3, 0145, 0146 §4, 0147 §4, 0174, 0182, 0184 §2,
0186 §8, 0192 §4, 0196 §3, 0204 §7, 0207 §3, 0224, 0258 §2, 0259 §2), in IEEE double arithmetic in the same order of
operations, with Python's own math; the handlers must meet them bit for bit. A text's mirror is CAD-EDIT-06's (its
MIRRTEXT policy): the texts are moved, turned and scaled here, not mirrored.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import (  # noqa: E402  (beside this file)
    apply, assert_no_negative_zero, js_hypot, linear, mirror, moved, norm_angle, reflects, rotation, scale_of, scaling,
    text_turn, translation,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.transform.kinds.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE),
            "children": []}


def p(x, y):
    return {"x": x, "y": y}


def base(i, layer="cizim", **extra):
    return {"id": i, "layerId": layer, "attrs": {}, **extra}


BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5101"
ENTITIES = [
    {"kind": "point", **base(1), "p": p(1.25, 0.5), "z": 0.75, "parts": [{"p": p(2.5, 0.5), "z": 0.8}]},
    {"kind": "line", **base(2, lineWeight=0.35), "a": p(0.0, 0.0), "b": p(4.0, 0.0), "za": 0.1, "zb": 0.2, "ma": 0.0, "mb": 4.0},
    {"kind": "polyline", **base(3), "pts": [p(0.0, 1.0), p(2.0, 1.5), p(3.25, 1.0)], "bulges": [0.5, 0.0], "zs": [0.1, None, 0.3],
     "ms": [0.0, None, 7.5]},
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
    {"kind": "dimension", **base(12, "olcu"), "a": p(0.0, 0.0), "b": p(4.0, 0.0), "offset": 0.5, "height": 0.18, "arrowSize": 1.2,
     "extOffset": 0.4},
    {"kind": "dimension", **base(13, "olcu"), "a": p(0.0, 0.0), "b": p(3.0, 4.0), "offset": -0.75, "height": 0.18, "style": "slope",
     "za": 1.0, "zb": 1.5},
    {"kind": "hatch", **base(14, "tarama"), "ring": [p(0.0, 8.0), p(2.0, 8.0), p(2.0, 9.0)],
     "holes": [[p(0.5, 8.2), p(1.0, 8.2), p(1.0, 8.5)]], "pattern": {"type": "lines", "angle": 45.0, "spacing": 0.2}},
    {"kind": "hatch", **base(15, "tarama"), "ring": [p(3.0, 8.0), p(5.0, 8.0), p(5.0, 9.0)],
     "pattern": {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": "ANSI31", "scale": 0.05,
                 "lines": [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 3.175], "dashes": [0.5, -0.25]}]}},
    {"kind": "hatch", **base(16, "tarama"), "ring": [p(6.0, 8.0), p(7.0, 8.0), p(7.0, 9.0)],
     "pattern": {"type": "solid", "angle": 0.0, "spacing": 1.0}},
    {"kind": "insert", **base(17), "attrs": {"NO": "K-1"}, "block": BLOCK, "p": p(10.0, 1.0), "scale": 2.0, "rotation": 0.3},
    {"kind": "leader", **base(18, "yazi"), "pts": [p(9.0, 5.0), p(10.0, 6.0)], "text": "Not", "height": 0.2, "rotation": 0.0,
     "arrowSize": 1.5},
    {"kind": "table", **base(19, "yazi"), "p": p(11.0, 9.0), "rotation": 0.0, "height": 0.2, "rows": [0.5, 0.5],
     "columns": [1.0, 2.25], "cells": [["A", "B"], ["1", "2"]], "frame": 0.05},
    {"kind": "image", **base(20), "p": p(12.0, 0.0), "width": 2.0, "height": 1.5, "rotation": 0.1, "file": "foto.png",
     "clip": [p(0.0, 0.0), p(1.0, 0.0), p(0.5, 1.0)], "opacity": 0.5},
    {"kind": "raster", **base(21), "affine": [20.0, 0.05, 0.0, 10.0, 0.0, -0.05], "width": 200, "height": 100, "bands": 1,
     "sample": "u8", "file": "tarama.tif", "srid": 0, "style": {"render": "gray", "bands": [1]}},
    {"kind": "pointcloud", **base(22), "sources": [{"file": "bulut.laz", "format": "laz", "count": 1000,
                                                    "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0]}],
     "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0], "count": 1000, "srid": 0, "style": {"render": "rgb", "size": 2}},
    {"kind": "nurbs", **base(23), "degree": 2, "ctrl": [p(0.0, 10.0), p(1.0, 11.0), p(2.0, 10.0), p(3.0, 11.5)],
     "knots": [0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0], "weights": [1.0, 0.5, 1.0, 1.0], "fit": [p(0.0, 10.0), p(3.0, 11.5)],
     "fitTolerance": 0.01, "tangents": [p(0.6, 0.8), None]},
    {"kind": "ray", **base(24), "p": p(4.0, 10.0), "dir": p(-1.0, 0.0)},
    {"kind": "polyline", **base(25), "pts": [p(5.0, 10.0), p(7.0, 10.0), p(7.0, 11.0)], "closed": True,
     "widths": [[0.2, 0.1], [0.1, 0.1], [0.0, 0.3]]},
    {"kind": "polyline", **base(26), "pts": [p(8.0, 10.0), p(9.0, 10.5)], "bulges": [0.25],
     "parts": [{"pts": [p(8.0, 11.0), p(9.5, 11.0)], "bulges": [-0.5]}]},
]
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}
TEXTS = {10, 11}
# Angles read back with atan2 from points the reference places with the platform's cos and sin: an arc's ends, an image's
# frame. The core's libm (fdlibm) and the platform's may round them an ulp apart; their steps carry `within` (10⁻¹²).
LIBM = {6, 20}
CLOUD = 22

BLOCKS = [{
    "id": BLOCK,
    "name": "Kapı",
    "base": p(0.1, 0.2),
    "entities": [
        {"kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": p(0.0, 0.0), "b": p(0.9, 0.0)},
        {"kind": "arc", "id": 2, "layerId": "cizim", "attrs": {}, "c": p(0.0, 0.0), "r": 0.9, "a0": 0.0, "a1": 1.5707963267948966},
    ],
    "attributes": [{"tag": "NO", "p": p(0.45, 0.3), "height": 0.15, "rotation": 0.0}],
}]


def setup(entities=None):
    return {
        "format": "kentos.document", "version": 1, "name": "Her tür",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(5.0, 5.0),
        "layers": [node("cizim", "Çizim"), node("yazi", "Yazı"), node("olcu", "Ölçü"), node("tarama", "Tarama")],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES if entities is None else entities),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── What a similarity does to every kind ──────────────────────────────────────────────────────────────────────────

def turn_of(a):
    t = math.fmod(a, 360.0)
    return t + 360.0 if t < 0.0 else t


def xy(v):
    return (v["x"], v["y"])


def pt(t):
    return {"x": t[0], "y": t[1]}


def transformed(e, m, new_id=None):
    """`e` under the similarity `m`, by the rules of its kind (the ADRs in this file's notes)."""
    k = e["kind"]
    s = scale_of(m)
    flip = reflects(m)
    if k in ("line", "circle", "arc", "ellipse", "spline", "dimension", "leader", "image", "raster") or (
            k in ("polygon",)) or (k == "hatch" and e["pattern"]["type"] in ("lines", "cross")):
        return moved(e, m, new_id)
    out = json.loads(json.dumps(e))
    if new_id is not None:
        out["id"] = new_id
    if k == "point":
        # Every point of a multi-point object, its elevation kept (docs/adr/0174).
        out["p"] = pt(apply(m, xy(e["p"])))
        if "parts" in e:
            out["parts"] = [{**q, "p": pt(apply(m, xy(q["p"])))} for q in e["parts"]]
    elif k == "polyline":
        # Rings as an area's (every part too); widths are lengths (docs/adr/0258 §2); measures and elevations stay.
        def ring(r):
            r["pts"] = [pt(apply(m, xy(q))) for q in r["pts"]]
            if "bulges" in r and flip:
                r["bulges"] = [-b for b in r["bulges"]]

        ring(out)
        for part in out.get("parts") or []:
            ring(part)
        if "widths" in out:
            out["widths"] = [[a * s, b * s] for a, b in e["widths"]]
    elif k in ("xline", "ray"):
        d = linear(m, xy(e["dir"]))
        length = js_hypot(*d) or 1.0
        out["p"] = pt(apply(m, xy(e["p"])))
        out["dir"] = pt((d[0] / length, d[1] / length))
    elif k == "nurbs":
        # Affine-invariant (docs/adr/0259 §2): control and fit points mapped, knots and weights kept, the tolerance
        # scaled, tangents mapped and made unit.
        out["ctrl"] = [pt(apply(m, xy(q))) for q in e["ctrl"]]
        if "fit" in e:
            out["fit"] = [pt(apply(m, xy(q))) for q in e["fit"]]
        if "fitTolerance" in e:
            out["fitTolerance"] = e["fitTolerance"] * s
        if "tangents" in e:
            def unit(t):
                if t is None:
                    return None
                d = linear(m, xy(t))
                length = math.sqrt(d[0] * d[0] + d[1] * d[1])
                return pt((d[0] / length, d[1] / length)) if length > 0.0 else pt(d)
            out["tangents"] = [unit(t) for t in e["tangents"]]
    elif k == "text":
        assert not flip, "a text's mirror is CAD-EDIT-06's"
        out["p"] = pt(apply(m, xy(e["p"])))
        out["height"] = e["height"] * s
        if "boxWidth" in e:
            out["boxWidth"] = e["boxWidth"] * s
        # Along a curve (docs/adr/0196 §3): its frame turns as a text's turns (from 0 up to 360), its curve scales,
        # its bulges stay.
        out["rotation"] = text_turn(e["rotation"], m)
        if "path" in e:
            out["path"]["pts"] = [pt((q["x"] * s, q["y"] * s)) for q in e["path"]["pts"]]
    elif k == "hatch":
        kind = e["pattern"]["type"]
        out["ring"] = [pt(apply(m, xy(q))) for q in e["ring"]]
        if "holes" in e:
            out["holes"] = [[pt(apply(m, xy(q))) for q in h] for h in e["holes"]]
        if kind in ("pattern", "gradient"):
            # docs/adr/0186 §8: turned by the map's turn, its lengths times the scale, reflected first in its x axis.
            x = linear(m, (1.0, 0.0))
            turn = (math.atan2(x[1], x[0]) * 180.0) / math.pi
            pat = out["pattern"]
            pat["angle"] = turn_of(turn - e["pattern"]["angle"] if flip else turn + e["pattern"]["angle"])
            if kind == "pattern":
                if "scale" in e["pattern"]:
                    pat["scale"] = e["pattern"]["scale"] * s
                if flip and "lines" in e["pattern"]:
                    pat["lines"] = [{**l, "angle": turn_of(-l["angle"]), "origin": [l["origin"][0], -l["origin"][1]],
                                     "offset": [l["offset"][0], -l["offset"][1]]} for l in e["pattern"]["lines"]]
    elif k == "insert":
        # docs/adr/0144 §3: the insertion point moves, the scale takes the length scale, the turn adds the map's; a
        # reflection flips the mirror and runs the turn backwards.
        theta = math.atan2(m[1], m[0])
        out["p"] = pt(apply(m, xy(e["p"])))
        out["scale"] = e["scale"] * s
        out["rotation"] = norm_angle(theta - e["rotation"] if flip else theta + e["rotation"])
        if e.get("mirror", False) != flip:
            out["mirror"] = True
        else:
            out.pop("mirror", None)
    elif k == "table":
        # docs/adr/0184 §2: its corner moves (mirrored, its top right's image is its new top left), it turns as a text
        # does, its sizes scale.
        corner = xy(e["p"])
        if flip:
            width = sum(e["columns"])
            r = (e["rotation"] * math.pi) / 180.0
            corner = (corner[0] + math.cos(r) * width, corner[1] + math.sin(r) * width)
        out["p"] = pt(apply(m, corner))
        out["rotation"] = text_turn(e["rotation"], m)
        out["height"] = e["height"] * s
        out["rows"] = [h * s for h in e["rows"]]
        out["columns"] = [w * s for w in e["columns"]]
        if "frame" in e:
            out["frame"] = e["frame"] * s
    else:
        raise ValueError(k)
    return out


def clean(v):
    """−0 written as 0: JSON cannot hold JavaScript's −0, and the two compare equal as numbers."""
    if isinstance(v, float) and v == 0.0:
        return 0.0
    if isinstance(v, dict):
        return {k: clean(x) for k, x in v.items()}
    if isinstance(v, list):
        return [clean(x) for x in v]
    return v


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────

def U(i):
    return f"$uidOf:{i}"


MOVE = {"kind": "move", "dx": 12.5, "dy": -7.25}
ROTATE = {"kind": "rotate", "center": p(1.0, 1.0), "angle": 0.5}
SCALE = {"kind": "scale", "center": p(1.0, 1.0), "factor": 2.5}
MIRROR = {"kind": "mirror", "a": p(2.0, -1.0), "b": p(5.0, 3.0)}
MAPS = [
    ("taşı", MOVE, translation(12.5, -7.25)),
    ("döndür", ROTATE, rotation(0.5, (1.0, 1.0))),
    ("ölçekle", SCALE, scaling(2.5, (1.0, 1.0))),
    ("aynala", MIRROR, mirror((2.0, -1.0), (5.0, 3.0))),
]
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
FIXED = "Nokta bulutu taşınmaz, döndürülmez, ölçeklenmez, aynalanmaz ve kopyalanmaz: konumu dosyasındadır. Bulutu seçimden çıkarın."
NAMES = {
    1: "nokta: ek noktaları ve kotları", 2: "çizgi: kotları ve ölçüleri (M) kalır", 3: "çoklu çizgi: kavisleri, kotları ve ölçüleri",
    4: "alan: delikleri ve parçaları, deliğin kotları", 5: "daire", 6: "yay: saat yönünün tersine kalır",
    7: "eliptik yay: parametreleri aynada döner", 8: "eğri", 9: "yardımcı çizgi: doğrultusu birim kalır",
    10: "çok satırlı yazı: kutusu ölçeklenir, satır aralığı ve genişlik çarpanı kalır",
    11: "eğri boyunca yazı: çerçevesi döner (saat yönünde de 0'dan 360'a), eğrisi ölçeklenir",
    12: "hizalı ölçü: ok boyu ve uzatma payı kalır, aynada uzaklık işaret değiştirir", 13: "eğim ölçüsü: kotları kalır",
    14: "çizgi taraması: deliği, açısı yarım tura göre, aralığı ölçekle", 15: "desenli tarama: açısı ve ölçeği, aynada aileleri",
    16: "dolu tarama", 17: "blok: ölçeği, dönüşü, aynası ve öznitelikleri", 18: "kılavuz: notu yazı gibi döner, ok boyu kalır",
    19: "tablo: köşesi, satır ve sütunları, çerçevesi", 20: "resim: çerçevesi ve kırpması", 21: "raster: dönüşümü birleşir",
    23: "NURBS eğrisi: denetim ve geçiş noktaları, teğetleri, toleransı", 24: "ışın", 25: "kapalı ve genişlikli çoklu çizgi: genişlikler ölçeklenir",
    26: "çok parçalı çoklu çizgi: her parça",
}


def plan_step(transform, entities, copy_=False):
    t = {"uids": [U(i) for i, _ in entities], "transform": transform}
    if copy_:
        t["copy"] = True
    return {"op": "plan", "input": t,
            "result": {"status": "completed", "output": {"sources": [U(i) for i, _ in entities],
                                                         "entities": [e for _, e in entities], "locked": [],
                                                         "revision": "$current"}, "warnings": []},
            "expect": SAME}


def kind_cases():
    out = []
    for i in IDS:
        if i == CLOUD:
            continue
        e = BY_ID[i]
        steps = []
        for name, t, m in MAPS:
            if name == "aynala" and i in TEXTS:
                continue
            got = clean(transformed(e, m))
            assert_no_negative_zero(got, f"{i}/{name}")
            step = plan_step(t, [(i, got)])
            if i in LIBM:
                step["within"] = 1e-12
            steps.append(step)
        if i in TEXTS:
            # Clockwise: the frame's turn is written from 0 up to 360, as a text's (docs/adr/0196 §3).
            back = {"kind": "rotate", "center": p(1.0, 1.0), "angle": -1.0}
            steps.append(plan_step(back, [(i, clean(transformed(e, rotation(-1.0, (1.0, 1.0)))))]))
        out.append({"name": f"her tür: {NAMES[i]}; taşı, döndür, ölçekle" + ("" if i in TEXTS else ", aynala"), "steps": steps})
    return out


def other_cases():
    out = []
    # The copies: one each, new ids after the drawing's last (26); the originals stay; one undo step.
    ids = [2, 5, 17, 19, 23]
    made = [(27 + k, transformed(BY_ID[i], translation(12.5, -7.25), 27 + k)) for k, i in enumerate(ids)]
    out.append({"name": "kopya sayısı: beş türün birer kopyası yeni kimliklerle, asıllar yerinde; tek adımda geri alınır; adım Kopyala",
                "steps": [
                    {"op": "execute", "input": {"uids": [U(i) for i in ids], "transform": MOVE, "copy": True},
                     "result": {"status": "completed", "output": {"changed": [], "created": [f"$uidOf:{s}" for s, _ in made],
                                                                  "locked": [], "revision": "$current"}, "warnings": []},
                     "expect": {"ids": IDS + [s for s, _ in made], "entities": {**{str(s): c for s, c in made},
                                                                                **{str(i): BY_ID[i] for i in ids}},
                                "revision": "changed"}},
                    {"op": "undo", "returns": "Kopyala", "expect": {"ids": IDS}},
                ]})
    # The point cloud: its place is its files'.
    for name, t, _ in MAPS:
        out.append({"name": f"nokta bulutu: {name} reddedilir, pointcloud_fixed; ötekiler de yazılmaz",
                    "steps": [{"op": "execute", "input": {"uids": [U(2), U(CLOUD)], "transform": t},
                               "result": {"status": "failed", "error": {"code": "pointcloud_fixed", "message": FIXED,
                                                                        "path": "uids[1]"}},
                               "expect": SAME}]})
    # Several kinds in one turn, in the input's order.
    pairs = [(i, clean(transformed(BY_ID[i], MAPS[1][2]))) for i in (17, 4, 25)]
    out.append({"name": "birden çok tür tek dönüşümde: blok, delikli alan, kapalı genişlikli çoklu çizgi; girdinin sırasıyla",
                "steps": [plan_step(ROTATE, pairs)]})
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.entities.transform", "commandVersion": 1,
        "title": "Her nesne türü: taşı, döndür, ölçekle, aynala; kopya sayısı; izleyen nesneler",
        "note": "ADR 0291. Çizimde her türden bir nesne (ve türün tuttukları); her biri dört dönüşümle planlanır (yazılmaz): "
                "taşı (12.5, -7.25), (1, 1) çevresinde 0,5 radyan, (1, 1) çevresinde 2,5 kat, (2, -1)–(5, 3) ekseninde ayna. "
                "Yazının aynası CAD-EDIT-06'nındır. Nokta bulutu reddedilir. Bağımsız başvuru: "
                "scripts/fixtures/transform_kinds_cases.py (affine_reference.py ile).",
        "setup": setup(), "cases": kind_cases() + other_cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"her türün dönüşüm durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
