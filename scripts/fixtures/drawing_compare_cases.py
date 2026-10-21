#!/usr/bin/env python3
"""The shared cases of cad.drawing.compare (docs/adr/0576 §3; plan/gis.md CBS-19), written from the ADR without KentOS
code, and the drawing files they compare with, written by the independent KCAD v2 writer (kcad_v2_reference.py).

    python3 scripts/fixtures/drawing_compare_cases.py           # writes fixtures/commands/v1/cad.drawing.compare.json
                                                                # and fixtures/compare/v1/revisions/*.kcad
    python3 scripts/fixtures/drawing_compare_cases.py --check   # writes nothing; compares

The rule: the objects of the two states are paired by their persistent ids; one only in the open drawing is added, one
only in the other state removed; a pair is compared by the data compare rules of docs/adr/0179 (written here from the
ADR): the geometry is the same when the kind and the number of vertices agree, the points (a path's or ring's
vertices and the middle of each edge, at the best alignment: a path either way round, a ring from any start either
way round) lie within the tolerance and a circle's radii differ by no more; a pair of another structure has the
distance between the centres of the boxes round their points, a pair of other kinds none. The attributes, with the
layer as one more field, are equal or not (a missing one is empty). The states: the open drawing is a parcel map; the
history cases edit it and ask for the steps back, the file cases ask for a file written beside.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as kr  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.drawing.compare.json"
FILES = ROOT / "fixtures/compare/v1/revisions"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
X0, Y0 = 500000.0, 4400000.0
TOLERANCE = 0.001  # the command's default, metres


def uid(n):
    return f"0192f5a3-3333-7000-8000-{n:012d}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": STYLE,
            "children": []}


def square(id_, x, y, side, layer_id, attrs):
    pts = [{"x": x, "y": y}, {"x": x + side, "y": y}, {"x": x + side, "y": y + side}, {"x": x, "y": y + side}]
    return {"kind": "polygon", "id": id_, "layerId": layer_id, "attrs": attrs, "pts": pts}


def base():
    """The open drawing: three parcels, a line, a polyline, a point, a circle."""
    entities = [
        square(1, X0, Y0, 20.0, "parsel", {"No": "101", "Sahip": "A"}),
        square(2, X0 + 30, Y0, 20.0, "parsel", {"No": "102", "Sahip": "B"}),
        square(3, X0 + 60, Y0, 20.0, "parsel", {"No": "103"}),
        {"kind": "line", "id": 4, "layerId": "yol", "attrs": {}, "a": {"x": X0, "y": Y0 + 30}, "b": {"x": X0 + 40, "y": Y0 + 30}},
        {"kind": "polyline", "id": 5, "layerId": "yol", "attrs": {"Ad": "1. Sokak"},
         "pts": [{"x": X0, "y": Y0 + 40}, {"x": X0 + 30, "y": Y0 + 40}, {"x": X0 + 30, "y": Y0 + 60}]},
        {"kind": "point", "id": 6, "layerId": "yol", "attrs": {}, "p": {"x": X0 + 10, "y": Y0 + 50}},
        {"kind": "circle", "id": 7, "layerId": "yol", "attrs": {"Tür": "Ağaç"}, "c": {"x": X0 + 50, "y": Y0 + 50}, "r": 5.0},
    ]
    return {"format": "kentos.document", "version": 2, "name": "Revizyon karşılaştırma",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000.0},
            "origin": {"x": X0, "y": Y0}, "layers": [layer("parsel", "Parsel"), layer("yol", "Yol")], "activeLayer": "parsel",
            "entities": entities, "uids": [uid(e["id"]) for e in entities], "styles": {"items": [], "categories": []}}


# ── A model of the drawing: an object by its persistent id ────────────


class Model:
    """Objects in drawing order: (uid, entity)."""

    def __init__(self, d):
        self.items = [(u, copy.deepcopy(e)) for u, e in zip(d["uids"], d["entities"])]

    def get(self, u):
        return next(e for k, e in self.items if k == u)

    def move(self, u, dx, dy):
        e = self.get(u)
        for key in ("a", "b", "p", "c"):
            if key in e:
                e[key] = {"x": e[key]["x"] + dx, "y": e[key]["y"] + dy}
        if "pts" in e:
            e["pts"] = [{"x": p["x"] + dx, "y": p["y"] + dy} for p in e["pts"]]

    def attrs(self, u, change):
        a = self.get(u)["attrs"]
        for k, v in change.items():
            if v is None:
                a.pop(k, None)
            else:
                a[k] = v

    def layer(self, u, layer_id):
        self.get(u)["layerId"] = layer_id

    def remove(self, u):
        self.items = [(k, e) for k, e in self.items if k != u]

    def add(self, u, e):
        self.items.append((u, e))


# ── The data compare rules (docs/adr/0179), for the kinds the drawing holds ──


def pts_of(e):
    if e["kind"] == "polygon" or e["kind"] == "polyline":
        return [(p["x"], p["y"]) for p in e["pts"]]
    if e["kind"] == "line":
        return [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
    if e["kind"] == "point":
        return [(e["p"]["x"], e["p"]["y"])]
    return [(e["c"]["x"], e["c"]["y"])]


def mids(vs, closed):
    n = len(vs)
    return [((vs[i][0] + vs[(i + 1) % n][0]) / 2, (vs[i][1] + vs[(i + 1) % n][1]) / 2) for i in range(n if closed else n - 1)]


def dist(p, q):
    return math.hypot(p[0] - q[0], p[1] - q[1])


def spread(a, b, closed):
    """The largest distance between the matching points (vertices and edge middles) at the best alignment."""
    options = [b, b[::-1]]
    if closed:
        options = [o[s:] + o[:s] for o in options for s in range(len(b))]
    best = None
    for o in options:
        ds = [dist(p, q) for p, q in zip(a, o)] + [dist(p, q) for p, q in zip(mids(a, closed), mids(o, closed))]
        worst = max(ds)
        best = worst if best is None else min(best, worst)
    return best


def box_centre(vs):
    xs, ys = [v[0] for v in vs], [v[1] for v in vs]
    return ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)


def geometry(a, b, tolerance=TOLERANCE):
    """(the location difference, whether the geometry is the same)."""
    if a["kind"] != b["kind"]:
        return None, False
    pa, pb = pts_of(a), pts_of(b)
    if a["kind"] in ("point", "circle"):
        d = dist(pa[0], pb[0])
        same = d <= tolerance and (a["kind"] == "point" or abs(a["r"] - b["r"]) <= tolerance)
        return d, same
    if len(pa) != len(pb):
        return dist(box_centre(pa), box_centre(pb)), False
    d = spread(pa, pb, a["kind"] == "polygon")
    return d, d <= tolerance


def fields(a, b):
    """The attributes that differ, the layer one of them, by their codes' order (the layer's name starts with NUL)."""
    ka = {**a["attrs"], "\0layer": a["layerId"]}
    kb = {**b["attrs"], "\0layer": b["layerId"]}
    out = [k for k in sorted(set(ka) | set(kb)) if ka.get(k, "") != kb.get(k, "")]
    return ["layer" if k == "\0layer" else k for k in out]


def compare(old, new, limit=1000, tolerance=TOLERANCE):
    """The command's answer (without `against` and `revision`) for an old model and the new one."""
    olds = {u: e for u, e in old.items}
    news = {u: e for u, e in new.items}
    same, changed = 0, []
    for u, e in new.items:
        if u not in olds:
            continue
        d, geom_same = geometry(olds[u], e, tolerance)
        f = fields(olds[u], e)
        if geom_same and not f:
            same += 1
            continue
        status = "both" if (not geom_same and f) else ("geometry" if not geom_same else "attributes")
        item = {"uid": u, "status": status, "fields": f}
        if d is not None:
            item["distance"] = d
        changed.append(item)
    added = [u for u, _ in new.items if u not in olds]
    removed = [u for u, _ in old.items if u not in news]
    return {"same": same, "changed": len(changed), "added": len(added), "removed": len(removed),
            "addedUids": added[:limit], "removedUids": removed[:limit], "changedObjects": changed[:limit],
            "truncated": len(added) > limit or len(removed) > limit or len(changed) > limit}


# ── Cases ─────────────────────────────────────────────────────────────

Q = {"expect": {"revision": "same"}}


def answer(old, new, against, **kw):
    out = {"against": against, **compare(old, new, **kw), "revision": "$current"}
    return {"status": "completed", "output": out, "warnings": []}


answer_with = answer


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step_move(u, dx, dy):
    return {"op": "execute", "command": "cad.entities.transform",
            "input": {"uids": [u], "transform": {"kind": "move", "dx": dx, "dy": dy}},
            "result": {"status": "completed", "output": {"changed": [u], "created": [], "locked": [], "revision": "$current"}, "warnings": []}}


def step_attrs(u, attrs, slot):
    return {"op": "execute", "command": "cad.entities.set", "input": {"uids": [u], "attrs": attrs, "operation": "attributes"},
            "result": {"status": "completed", "output": {"changed": [u], "ids": [slot], "revision": "$current"}, "warnings": []}}


def step_layer(u, layer_id, slot):
    return {"op": "execute", "command": "cad.entities.set", "input": {"uids": [u], "layerId": layer_id, "operation": "layer"},
            "result": {"status": "completed", "output": {"changed": [u], "ids": [slot], "revision": "$current"}, "warnings": []}}


def step_delete(u):
    return {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [u]},
            "result": {"status": "completed", "output": {"removed": [u], "locked": [], "revision": "$current"}, "warnings": []}}


def step_line(slot, a, b, layer_id="yol"):
    return {"op": "execute", "command": "cad.line.create",
            "input": {"layerId": layer_id, "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}},
            "result": {"status": "completed", "output": {"uid": "$uid", "id": slot, "revision": "$current"}, "warnings": []}}


def ask(input_, result, within=1e-9):
    return {"op": "execute", "input": input_, "result": result, "within": within, **Q}


def history(steps, tolerance=None, limit=None):
    out = {"against": {"kind": "history", "steps": steps}}
    if tolerance is not None:
        out["tolerance"] = tolerance
    if limit is not None:
        out["limit"] = limit
    return out


def file_input(name, tolerance=None, limit=None):
    out = {"against": {"kind": "file", "path": f"$fixture:compare/v1/revisions/{name}"}}
    if tolerance is not None:
        out["tolerance"] = tolerance
    if limit is not None:
        out["limit"] = limit
    return out


def revision_files():
    """The drawing files the file cases compare with: (name, v2 JSON)."""
    out = {}
    d = base()
    out["ayni.kcad"] = d
    # A moved parcel, a parcel the open drawing has not (103 gone), an object it has not (a new line, uid 9).
    m = Model(d)
    m.move(uid(2), -5.0, 0.0)
    m.remove(uid(3))
    m.add(uid(9), {"kind": "line", "id": 8, "layerId": "yol", "attrs": {}, "a": {"x": X0, "y": Y0 + 70}, "b": {"x": X0 + 10, "y": Y0 + 70}})
    out["tasindi.kcad"] = from_model(d, m)
    # Attributes and a layer only.
    m = Model(d)
    m.attrs(uid(1), {"Sahip": "Z", "Yok": "var"})
    m.attrs(uid(2), {"Sahip": None})
    m.layer(uid(3), "yol")
    out["oznitelik.kcad"] = from_model(d, m)
    # Another structure (a parcel with a fifth vertex, centre of its box moved), another kind (the line a point).
    m = Model(d)
    p = m.get(uid(1))
    p["pts"] = p["pts"] + [{"x": X0 - 4.0, "y": Y0 + 10}]
    m.items = [(u, e) for u, e in m.items if u != uid(4)] + [(uid(4), {"kind": "point", "id": 7, "layerId": "yol", "attrs": {}, "p": {"x": X0 + 1.0, "y": Y0 + 30}})]
    out["yapi.kcad"] = from_model(d, m)
    # Moves under the tolerance and over it; a circle's radius.
    m = Model(d)
    m.move(uid(1), 0.0004, 0.0)
    m.move(uid(6), 0.0, 0.002)
    m.get(uid(7))["r"] = 5.0004
    out["kucuk.kcad"] = from_model(d, m)
    # A ring written from another start and the other way round, a path reversed: the same drawing.
    m = Model(d)
    p = m.get(uid(2))
    p["pts"] = p["pts"][2:] + p["pts"][:2]
    pl = m.get(uid(5))
    pl["pts"] = pl["pts"][::-1]
    out["donuk.kcad"] = from_model(d, m)
    return out


def from_model(d, m):
    out = copy.deepcopy(d)
    out["entities"] = [dict(e, id=i + 1) for i, (_, e) in enumerate(m.items)]
    out["uids"] = [u for u, _ in m.items]
    return out


def cases(files):
    d = base()
    now = Model(d)
    out = []

    def add(name, steps):
        out.append({"name": name, "steps": steps})

    def edited(edit):
        m = Model(d)
        edit(m)
        return m

    def ask_history(new, steps, back, label, **kw):
        against = f"{back} adım geri, “{label}” öncesi"
        return ask(history(back, **{k: v for k, v in kw.items() if k in ("tolerance", "limit")}),
                   answer(Model(d), new, against, **({"limit": kw["limit"]} if "limit" in kw else {})))

    # History: one step back for each kind of edit.
    moved = edited(lambda m: m.move(uid(2), 5.0, 0.0))
    add("Bir adım geri: 5 m taşınan parsel değişmiş, geometri; öbürleri aynı",
        [step_move(uid(2), 5.0, 0.0), ask_history(moved, 1, 1, "Taşı")])
    nudged = edited(lambda m: m.move(uid(2), 0.0004, 0.0))
    add("Bir adım geri: 0,4 mm'lik kayma tolerans içinde, hepsi aynı",
        [step_move(uid(2), 0.0004, 0.0), ask_history(nudged, 1, 1, "Taşı")])
    add("Tolerans 0: aynı 0,4 mm'lik kayma değişmiş, uzaklığı 0,0004",
        [step_move(uid(2), 0.0004, 0.0), {**ask(history(1, tolerance=0.0), answer_with(Model(d), nudged, "1 adım geri, “Taşı” öncesi", tolerance=0.0))}])
    dropped = edited(lambda m: m.remove(uid(4)))
    add("Silinen çizgi: şimdi olmayan, öbüründe olan silinmiş",
        [step_delete(uid(4)), ask_history(dropped, 1, 1, "Sil")])

    def draw(m):
        m.add("$uidOf:8", {"kind": "line", "id": 8, "layerId": "yol", "attrs": {}, "a": {"x": X0 + 5, "y": Y0 + 80}, "b": {"x": X0 + 25, "y": Y0 + 80}})

    add("Çizilen çizgi: şimdi olan, öbüründe olmayan eklenmiş",
        [step_line(8, (X0 + 5, Y0 + 80), (X0 + 25, Y0 + 80)), ask_history(edited(draw), 1, 1, "Ekle")])
    add("Öznitelikler: değişen, eklenen ve silinen alan; durum attributes, alanlar kod sırasıyla",
        [step_attrs(uid(1), {"Sahip": "C", "Yeni": "x", "No": None}, 1),
         ask_history(edited(lambda m: m.attrs(uid(1), {"Sahip": "C", "Yeni": "x", "No": None})), 1, 1, "Değiştir")])
    add("Katman değişti: öznitelik gibi, alanlarda layer",
        [step_layer(uid(3), "yol", 3), ask_history(edited(lambda m: m.layer(uid(3), "yol")), 1, 1, "Katman değiştir")])

    def both(m):
        m.move(uid(1), 0.0, 3.0)
        m.attrs(uid(1), {"Sahip": "Q"})

    add("İki adım geri: taşınan ve öznitelikleri değişen parsel both, tek nesne",
        [step_move(uid(1), 0.0, 3.0), step_attrs(uid(1), {"Sahip": "Q"}, 1), ask_history(edited(both), 2, 2, "Taşı")])

    def three(m):
        m.move(uid(2), 5.0, 0.0)
        m.remove(uid(6))
        m.add("$uidOf:8", {"kind": "line", "id": 8, "layerId": "yol", "attrs": {}, "a": {"x": X0, "y": Y0 + 90}, "b": {"x": X0 + 5, "y": Y0 + 90}})

    add("Üç adım: parsel taşındı, nokta silindi, çizgi çizildi; ilk adıma kadar hepsi",
        [step_move(uid(2), 5.0, 0.0), step_delete(uid(6)), step_line(8, (X0, Y0 + 90), (X0 + 5, Y0 + 90)),
         ask_history(edited(three), 3, 3, "Taşı")])
    add("Geri alınan adım geçmişte yok: 2 adım istenir, 1 var",
        [step_move(uid(2), 5.0, 0.0), step_move(uid(1), 1.0, 0.0), {"op": "undo", "returns": "Taşı"},
         {"op": "execute", "input": history(2), "result": refusal(
             "no_such_step", "Geri alma geçmişi 1 adım tutuyor; 2 adım geri istendi. 1 ile 1 arasında bir sayı verin.", "against.steps")},
         ask_history(moved, 1, 1, "Taşı")])
    add("Geçmişten fazla adım: ret",
        [step_move(uid(2), 5.0, 0.0), {"op": "execute", "input": history(3), "result": refusal(
            "no_such_step", "Geri alma geçmişi 1 adım tutuyor; 3 adım geri istendi. 1 ile 1 arasında bir sayı verin.", "against.steps")}])
    add("Hiç değişiklik yokken geri adım: ret",
        [{"op": "execute", "input": history(1), "result": refusal(
            "no_such_step", "Geri alma geçmişinde adım yok; önce çizimde bir değişiklik yapın.", "against.steps")}])
    add("Sıfır adım: ret",
        [step_move(uid(2), 5.0, 0.0), {"op": "execute", "input": history(0), "result": refusal(
            "no_such_step", "Geri alma geçmişi 1 adım tutuyor; 0 adım geri istendi. 1 ile 1 arasında bir sayı verin.", "against.steps")}])
    add("Tolerans eksi: ret",
        [step_move(uid(2), 5.0, 0.0), {"op": "execute", "input": history(1, tolerance=-1.0), "result": refusal(
            "invalid_tolerance", "Tolerans 0 ile 10 metre arasında olmalı.", "tolerance")}])
    add("Tolerans 10 m'den büyük: ret",
        [step_move(uid(2), 5.0, 0.0), {"op": "execute", "input": history(1, tolerance=10.5), "result": refusal(
            "invalid_tolerance", "Tolerans 0 ile 10 metre arasında olmalı.", "tolerance")}])
    add("Liste sınırı sıfır: ret",
        [step_move(uid(2), 5.0, 0.0), {"op": "execute", "input": history(1, limit=0), "result": refusal(
            "invalid_limit", "Liste sınırı 1 ile 1000000 arasında bir tam sayı olmalı.", "limit")}])

    def many(m):
        m.move(uid(1), 5.0, 0.0)
        m.move(uid(2), 5.0, 0.0)
        m.add("$uidOf:8", {"kind": "point", "id": 8, "layerId": "yol", "attrs": {}, "p": {"x": X0, "y": Y0 + 100}})
        m.add("$uidOf:9", {"kind": "point", "id": 9, "layerId": "yol", "attrs": {}, "p": {"x": X0 + 5, "y": Y0 + 100}})

    steps = [step_move(uid(1), 5.0, 0.0), step_move(uid(2), 5.0, 0.0)]
    for slot, x in ((8, X0), (9, X0 + 5)):
        steps.append({"op": "execute", "command": "cad.point.create", "input": {"layerId": "yol", "p": {"x": x, "y": Y0 + 100}},
                      "result": {"status": "completed", "output": {"uid": "$uid", "id": slot, "revision": "$current"}, "warnings": []}})
    steps.append(ask_history(edited(many), 4, 4, "Taşı", limit=1))
    add("Sınır 1: kesilen listeler truncated der, sayılar bütün", steps)

    # Files.
    drawing = {name: Model(f) for name, f in files.items()}
    for name, why in (("ayni.kcad", "Aynı çizim dosyası: hepsi aynı"),
                      ("tasindi.kcad", "Dosyada 5 m geride parsel, parsel 103 yok, bir çizgi daha var: değişen, eklenen ve silinen"),
                      ("oznitelik.kcad", "Dosyada başka öznitelikler ve katman: üç nesne değişmiş"),
                      ("yapi.kcad", "Dosyada beşinci köşeli parsel ve nokta olan çizgi: yapı ve tür farkı"),
                      ("kucuk.kcad", "Dosyada toleransın altındaki kaymalar: hepsi aynı, 0,002 m'lik kayma değişmiş"),
                      ("donuk.kcad", "Başka köşeden ve ters yönden yazılan halka ve yol: aynı çizim")):
        add(why, [ask(file_input(name), answer(drawing[name], now, name))])
    add("Dosyada küçük kaymalar, toleransla 0: üç nesne değişmiş",
        [ask(file_input("kucuk.kcad", tolerance=0.0), answer_with(drawing["kucuk.kcad"], now, "kucuk.kcad", tolerance=0.0))])
    add("Dosya, sınır 1: kesilen listeler",
        [ask(file_input("oznitelik.kcad", limit=1), answer(drawing["oznitelik.kcad"], now, "oznitelik.kcad", limit=1))])
    add("Göreli yol: ret",
        [{"op": "execute", "input": {"against": {"kind": "file", "path": "revizyon.kcad"}}, "result": refusal(
            "invalid_file_path", "“revizyon.kcad” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).", "against.path")}])
    add("Olmayan dosya: ret",
        [{"op": "execute", "input": file_input("yok.kcad"), "result": refusal(
            "file_not_found", "$fixture:compare/v1/revisions/yok.kcad okunamadı: dosya yok ya da açılamıyor.", "against.path")}])
    add("Bozuk dosya: ret",
        [{"op": "execute", "input": {"against": {"kind": "file", "path": "$fixture:kcad/v2/broken/arrays-items-short.kcad"}},
          "result": refusal("import_unreadable", "$any", "against.path")}])
    return out


def build(files):
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.drawing.compare", "commandVersion": 1,
            "title": "Revizyonları karşılaştır",
            "note": "ADR 0576. Bağımsız başvuru scripts/fixtures/drawing_compare_cases.py: nesneler kalıcı kimlikle eşlenir, farkı veri karşılaştırmanın "
                    "kuralıyla (ADR 0179) bulunur; dosyalar kcad_v2_reference.py'nin yazıcısıyla yazılır; KentOS kodu yok.",
            "setup": base(), "cases": cases(files)}


def main():
    files = revision_files()
    outputs = {FILES / name: kr.container(kr.document(doc)) for name, doc in files.items()}
    text = json.dumps(build(files), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        bad = [p for p, b in outputs.items() if not p.exists() or p.read_bytes() != b]
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            bad.append(OUT)
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if bad:
            return 1
        print("fixtures/commands/v1/cad.drawing.compare.json ve revisions/: güncel.")
        return 0
    FILES.mkdir(parents=True, exist_ok=True)
    for p, b in outputs.items():
        p.write_bytes(b)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
