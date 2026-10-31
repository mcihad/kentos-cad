#!/usr/bin/env python3
"""A named view's area edited (docs/adr/0789 §1; Revit's crop region grips and Size Crop), from the ADR's rule, without
KentOS code.

    python3 scripts/fixtures/view_area_cases.py           # writes the two files
    python3 scripts/fixtures/view_area_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.views.edit.area.json and cad.sheets.set.area.json. The rule:

- `cad.views.edit`'s `area` gives the view of that name (trimmed) a new middle, width and height and, when given, a twist
  (any turn, kept in [0, 2π): fmod's remainder, a negative one turned once); no twist keeps the view's own. Its id, name,
  place in the list and recipe (its version too) stay;
- the numbers are checked first, as Save's are (`not_finite`, `far_point`, `invalid_view_size`), then the name
  (`invalid_view_name`), the revision, then the view (`unknown_view`);
- the same area again changes nothing; another one is one undo step, “Görünümün alanı”: undo gives the old area back and
  redo the new one, to the view of the step's key (its id, else the one its name gave) while it is there; whatever else
  changed since (its name, its recipe, other views, all settings edits outside the undo history) stays;
- the maps linked to the view draw its area at once (their centre is the view's; a cropping recipe cuts them to the
  area's box), so `cad.sheets.set` after an area change (and after its undo) says what they show then (on top of
  ADR 0774's and 0781's references, `view_recipe_sheet_cases.py` and `view_recipe_crop_cases.py`).
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import view_recipe_crop_cases as crop  # noqa: E402
import view_recipe_sheet_cases as vrs  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT_VIEWS = ROOT / "fixtures/commands/v1/cad.views.edit.area.json"
OUT_SHEETS = ROOT / "fixtures/commands/v1/cad.sheets.set.area.json"
TAU = 2.0 * math.pi
E0, N0 = 487000, 4420000
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": x, "y": y}


def fnv(name):
    h = 0x811C9DC5
    for b in name.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"v-{h:08x}"


def norm(a):
    r = math.fmod(a, TAU)
    if r < 0.0:
        r += TAU
    if r >= TAU:
        r = 0.0
    return r + 0.0


def key(v):
    return v.get("id") or fnv(v["name"])


# ── The project of cad.views.edit.area.json ─────────────────────────────────────────────────────────────────────────

LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]
VIEWS = [
    {"id": "v-plan", "name": "Plan", "center": P(E0 + 4, N0 + 3), "width": 12.0, "height": 8.0, "twist": 0.0,
     "recipe": {"version": 2, "level": {"id": "zemin"}, "scale": 50, "crop": True}},
    {"name": "Detay", "center": P(E0 + 2, N0 + 1), "width": 3.0, "height": 2.0, "twist": 0.5},
    {"name": "Kuzey", "center": P(E0, N0), "width": 40.0, "height": 30.0, "twist": 0.0},
]
ENTITIES = [{"kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": P(E0, N0), "b": P(E0 + 8, N0 + 6)}]
IDS = [1]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Görünümün alanı",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "drawingFont": "barlow", "arch": {"levels": LEVELS}, "namedViews": VIEWS},
    "origin": P(E0, N0),
    "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": STYLE, "children": []}],
    "activeLayer": "cizim", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

NON_FINITE = "Görünümün sayıları sonlu olmalı; NaN ya da sonsuz bir değer verildi. Sayıyı yeniden verin."
FAR_TEXT = "Görünümün ortası çizimden çok uzak: koordinatlar en çok 10 000 km olur. Çizimdeki bir noktayı verin."
SIZE_TEXT = "Görünümün eni ve boyu 0'dan büyük ve en çok 20 000 km olmalı. Gösterilecek alanın boyunu verin."
NAME_TEXT = "Görünüm adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın."
CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
            "hazırlayın.")
REVISION = ("Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.")


def unknown_text(name):
    return f"“{name}” adlı bir görünüm yok. Adlı görünümlerden birini seçin ya da geçerli görünümü bu adla kaydedin."


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def area(name, center, w, h, twist=None):
    out = {"kind": "area", "name": name, "center": center, "width": w, "height": h}
    if twist is not None:
        out["twist"] = twist
    return out


class Project:
    """The views, the undo and redo stacks of area steps (key, before, after) and whether anything was edited."""

    def __init__(self):
        self.views = copy.deepcopy(VIEWS)
        self.undo = []
        self.redo = []
        self.dirty = False

    def expect(self, revision):
        return {"ids": IDS, "settings": {"namedViews": copy.deepcopy(self.views)}, "canUndo": bool(self.undo),
                "canRedo": bool(self.redo), "dirty": self.dirty, "revision": revision}

    def find(self, name):
        return next((i for i, v in enumerate(self.views) if v["name"] == name), None)

    def answer(self, edit):
        """What `area` would do: (result, next views, the step) or (refusal, None, None)."""
        for path, v in [("edit.center", edit["center"]["x"]), ("edit.center", edit["center"]["y"]),
                        ("edit.width", edit["width"]), ("edit.height", edit["height"]),
                        ("edit.twist", edit.get("twist", 0.0))]:
            if not math.isfinite(v):
                return failed("not_finite", NON_FINITE, path), None, None
        if abs(edit["center"]["x"]) > 1e7 or abs(edit["center"]["y"]) > 1e7:
            return failed("far_point", FAR_TEXT, "edit.center"), None, None
        for path, v in [("edit.width", edit["width"]), ("edit.height", edit["height"])]:
            if v <= 0.0 or v > 2e7:
                return failed("invalid_view_size", SIZE_TEXT, path), None, None
        name = edit["name"].strip()
        if not name or len(name) > 64:
            return failed("invalid_view_name", NAME_TEXT, "edit.name"), None, None
        at = self.find(name)
        if at is None:
            return failed("unknown_view", unknown_text(name), "edit.name"), None, None
        views = copy.deepcopy(self.views)
        v = views[at]
        before = {k: v[k] for k in ("center", "width", "height", "twist")}
        v["center"], v["width"], v["height"] = edit["center"], edit["width"], edit["height"]
        if "twist" in edit:
            v["twist"] = norm(edit["twist"])
        after = {k: v[k] for k in ("center", "width", "height", "twist")}
        changed = views != self.views
        output = {"views": copy.deepcopy(views), "view": copy.deepcopy(v), "changed": changed, "revision": "$current"}
        return {"status": "completed", "output": output, "warnings": []}, views, (key(v), before, after) if changed else None

    def run(self, edit, op="execute", **extra):
        step = {"op": op, "input": {"edit": edit, **extra}}
        result, views, done = self.answer(edit)
        if views is None:
            step["result"] = result
            step["expect"] = self.expect("same")
            return step
        if op == "plan":
            step["result"] = result
            step["expect"] = self.expect("same")
            return step
        if op == "validate":
            step["result"] = {"status": "completed", "output": None, "warnings": []}
            step["expect"] = self.expect("same")
            return step
        step["result"] = result
        if done is None:
            step["expect"] = self.expect("same")
            return step
        self.views = views
        self.undo.append(done)
        self.redo = []
        self.dirty = True
        step["expect"] = self.expect("changed")
        return step

    def put(self, k, a):
        for v in self.views:
            if key(v) == k:
                v.update(copy.deepcopy(a))

    def back(self, op):
        """Undo or redo: the step's area on the view of its key, while it is there."""
        k, before, after = (self.undo if op == "undo" else self.redo).pop()
        self.put(k, before if op == "undo" else after)
        (self.redo if op == "undo" else self.undo).append((k, before, after))
        return {"op": op, "expect": self.expect("changed")}



def rename(p, name, to):
    at = p.find(name)
    v = p.views[at]
    if "id" not in v:
        v = {"id": fnv(name), **v}
    v["name"] = to
    p.views[at] = v
    step = {"op": "execute", "input": {"edit": {"kind": "rename", "name": name, "to": to}},
            "result": {"status": "completed", "output": {"views": copy.deepcopy(p.views), "view": copy.deepcopy(v),
                                                         "changed": True, "revision": "$current"}, "warnings": []}}
    p.dirty = True
    step["expect"] = p.expect("changed")
    return step


def delete(p, name):
    p.views = [v for v in p.views if v["name"] != name]
    p.dirty = True
    step = {"op": "execute", "input": {"edit": {"kind": "delete", "name": name}},
            "result": {"status": "completed", "output": {"views": copy.deepcopy(p.views), "changed": True,
                                                         "revision": "$current"}, "warnings": []}}
    step["expect"] = p.expect("changed")
    return step


def recipe(p, name, r):
    at = p.find(name)
    v = p.views[at]
    was = v.get("recipe", {}).get("version", 0)
    if "id" not in v:
        v = {"id": fnv(name), **v}
    v["recipe"] = {"version": was + 1, **r}
    p.views[at] = v
    p.dirty = True
    step = {"op": "execute", "input": {"edit": {"kind": "recipe", "name": name, "recipe": r}},
            "result": {"status": "completed", "output": {"views": copy.deepcopy(p.views), "view": copy.deepcopy(v),
                                                         "changed": True, "revision": "$current"}, "warnings": []}}
    step["expect"] = p.expect("changed")
    return step


def view_cases():
    out = []

    def add(name, build):
        assert name not in [c["name"] for c in out], name
        p = Project()
        out.append({"name": name, "steps": build(p)})

    here = P(E0 + 5, N0 + 4)
    add("Alanı değiştir: orta, en ve boy yazılır; kimlik, sıra ve tarif (sürümüyle) kalır; tek geri alma adımı",
        lambda p: [p.run(area("Plan", here, 14.0, 9.5))])
    add("Dönüş verilmezse görünümün kendi dönüşü kalır", lambda p: [p.run(area("Detay", P(E0 + 2.5, N0 + 1), 4.0, 2.0))])
    add("Eksi çeyrek tur [0, 2π) aralığına çevrilir",
        lambda p: [p.run(area("Detay", P(E0 + 2, N0 + 1), 3.0, 2.0, -math.pi / 2))])
    add("Tam tur dönüş sıfırdır", lambda p: [p.run(area("Detay", P(E0 + 2, N0 + 1), 3.0, 2.0, TAU))])
    add("Tam turdan fazlası indirgenir", lambda p: [p.run(area("Kuzey", P(E0, N0), 40.0, 30.0, 7.0))])
    add("Aynı alan yeniden: bir şey değişmez, geri alma adımı olmaz",
        lambda p: [p.run(area("Plan", P(E0 + 4, N0 + 3), 12.0, 8.0))])
    add("Aynı alan dönüşü yazılarak: değişmez", lambda p: [p.run(area("Detay", P(E0 + 2, N0 + 1), 3.0, 2.0, 0.5))])
    add("Geri al eski alanı verir, Yinele yenisini",
        lambda p: [p.run(area("Plan", here, 14.0, 9.5)), p.back("undo"), p.back("redo")])
    add("İki alan değişikliği iki adım: geri almalar sırayla döner",
        lambda p: [p.run(area("Plan", here, 14.0, 9.5)), p.run(area("Plan", here, 16.0, 9.5)), p.back("undo"),
                   p.back("undo")])
    add("Adın başındaki ve sonundaki boşluklar kırpılır", lambda p: [p.run(area("  Plan ", here, 10.0, 6.0))])
    add("Kimliksiz görünümün alanı değişir, kimlik yazılmaz",
        lambda p: [p.run(area("Kuzey", P(E0 + 10, N0 + 5), 20.0, 15.0))])
    add("Arada yeniden adlandırılan görünüme geri alma alanını yine verir (kalıcı kimlik)",
        lambda p: [p.run(area("Plan", here, 14.0, 9.5)), rename(p, "Plan", "Uygulama planı"), p.back("undo")])
    add("Kimliksiz görünüm adlandırılınca adının kimliğini alır; geri alma onu bulur",
        lambda p: [p.run(area("Kuzey", P(E0 + 10, N0 + 5), 20.0, 15.0)), rename(p, "Kuzey", "Vaziyet"), p.back("undo")])
    add("Arada değişen tarif geri almada kalır: yalnız alan döner",
        lambda p: [p.run(area("Plan", here, 14.0, 9.5)),
                   recipe(p, "Plan", {"level": {"id": "zemin"}, "scale": 20, "crop": True, "detail": "detailed"}),
                   p.back("undo")])
    add("Arada silinen görünüme geri alma bir şey vermez",
        lambda p: [p.run(area("Detay", P(E0 + 3, N0 + 1), 3.0, 2.0)), delete(p, "Detay"), p.back("undo")])
    add("Plan alanı söyler, bir şey yazmaz", lambda p: [p.run(area("Plan", here, 14.0, 9.5), op="plan")])
    add("Doğrulama tamam der, bir şey yazmaz", lambda p: [p.run(area("Plan", here, 14.0, 9.5), op="validate")])
    add("Bilinmeyen görünüm reddedilir", lambda p: [p.run(area("Kesit", here, 5.0, 5.0))])
    add("Boş ad reddedilir", lambda p: [p.run(area("   ", here, 5.0, 5.0))])
    add("Sıfır en reddedilir", lambda p: [p.run(area("Plan", here, 0.0, 5.0))])
    add("Eksi boy reddedilir", lambda p: [p.run(area("Plan", here, 5.0, -1.0))])
    add("20 000 km'den büyük en reddedilir", lambda p: [p.run(area("Plan", here, 3e7, 5.0))])
    add("Çok uzak orta reddedilir", lambda p: [p.run(area("Plan", P(2e7, N0), 5.0, 5.0))])
    add("Bilinmeyen görünümden önce boy denetlenir", lambda p: [p.run(area("Kesit", here, 0.0, 5.0))])

    def refused(p, edit, result, non_finite=None, **extra):
        step = {"op": "execute", "input": {"edit": edit, **extra}, "result": result, "expect": p.expect("same")}
        if non_finite:
            step["nonFinite"] = non_finite
        return step

    add("Sonlu olmayan en reddedilir",
        lambda p: [refused(p, area("Plan", here, 5.0, 5.0), failed("not_finite", NON_FINITE, "edit.width"),
                           {"edit.width": "NaN"})])
    add("Sonlu olmayan dönüş reddedilir",
        lambda p: [refused(p, area("Plan", here, 5.0, 5.0, 0.0), failed("not_finite", NON_FINITE, "edit.twist"),
                           {"edit.twist": "Infinity"})])
    add("Sonlu olmayan orta reddedilir",
        lambda p: [refused(p, area("Plan", P(2e7, N0), 5.0, 5.0), failed("not_finite", NON_FINITE, "edit.center"),
                           {"edit.center.x": "-Infinity"})])

    def revision(p):
        return [{"op": "captureRevision", "as": "r0"},
                refused(p, area("Plan", here, 5.0, 5.0), failed("invalid_revision", REVISION, "expectedRevision"),
                        expectedRevision="07"),
                p.run(area("Plan", here, 14.0, 9.5), expectedRevision="$r0"),
                refused(p, area("Plan", here, 5.0, 5.0),
                        {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                         "path": "expectedRevision", "revision": "$current"}},
                        expectedRevision="$r0")]
    add("Beklenen sürüm: geçersizi ve eskisi reddedilir, doğrusu yazar", revision)
    return out


class Turned(vrs.Book):
    """ADR 0774's book, a linked map turned with its view's twist (the view's twist in thousandths of a degree)."""

    def camera(self, item):
        out, view, missing = super().camera(item)
        if out is not None and view is not None:
            out["kind"]["view"]["rotation"] = round(view["twist"] * 180000 / math.pi) % 360000
        return out, view, missing


# ── cad.sheets.set after an area change ─────────────────────────────────────────────────────────────────────────────


def sheet_cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": copy.deepcopy(crop.PRELUDE) + steps})

    def moved(center, w, h, twist=None):
        views = copy.deepcopy(crop.VIEWS)
        dar = views[-1]
        dar["center"], dar["width"], dar["height"] = center, w, h
        if twist is not None:
            dar["twist"] = norm(twist)
        return views

    def edit(center, w, h, twist=None):
        e = {"kind": "area", "name": "Dar", "center": center, "width": w, "height": h}
        if twist is not None:
            e["twist"] = twist
        return {"op": "execute", "command": "cad.views.edit", "input": {"edit": e}, "result": vrs.ANY,
                "expect": {"revision": "changed", "canUndo": True}}

    step = vrs.step

    def book(views=None):
        return Turned(crop.VIEWS if views is None else views, crop.SHEETS)
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True}}
    redo = {"op": "redo", "expect": {"canUndo": True, "canRedo": False}}
    wide = moved(vrs.P(4, 1.5), 6.0, 2.0)
    add("Alan sağa genişleyince dar harita taramaya değer", [edit(vrs.P(4, 1.5), 6.0, 2.0), step(book(wide), [4])])
    add("Genişleyen alanın haritası yeni ortasından çizer", [edit(vrs.P(4, 1.5), 6.0, 2.0), step(book(wide))])
    add("Geri alınca dar harita taramayı yine göstermez",
        [edit(vrs.P(4, 1.5), 6.0, 2.0), undo, step(book(), [4])])
    add("Yinelenince yine gösterir", [edit(vrs.P(4, 1.5), 6.0, 2.0), undo, redo, step(book(wide), [4])])
    small = moved(vrs.P(2, 2), 0.5, 0.5)
    add("Alan zemin kolonuna daralınca yalnız onu gösterir", [edit(vrs.P(2, 2), 0.5, 0.5), step(book(small), [1, 3])])
    away = moved(vrs.P(4, 1.5), 1.0, 1.0)
    add("Alan kolondan uzaklaşınca dar harita onu göstermez", [edit(vrs.P(4, 1.5), 1.0, 1.0), step(book(away), [1])])
    add("Uzaklaşan alan geri alınınca kolon döner", [edit(vrs.P(4, 1.5), 1.0, 1.0), undo, step(book(), [1])])
    turned = moved(vrs.P(2, 1.5), 4.0, 0.4, math.pi / 2)
    add("Dönen dar alan kutusuyla sayılır: dikey şerit kolonu ve çizgiyi tutar",
        [edit(vrs.P(2, 1.5), 4.0, 0.4, math.pi / 2), step(book(turned), [1, 3, 4])])
    add("Dönen alanın haritası görünümün dönüşüyle çizer", [edit(vrs.P(2, 1.5), 4.0, 0.4, math.pi / 2), step(book(turned))])
    tall = moved(vrs.P(2, 1.5), 2.0, 5.0)
    add("Boy uzayınca bütün nesnelerden alanındakiler", [edit(vrs.P(2, 1.5), 2.0, 5.0), step(book(tall), vrs.IDS)])
    add("Genişleyen alanda bütün nesneler", [edit(vrs.P(4, 1.5), 6.0, 2.0), step(book(wide), vrs.IDS)])
    left = moved(vrs.P(0.5, 1.5), 2.0, 2.0)
    add("Alan batıya kayınca kolon kenarda kalır, tarama yine dışarıda",
        [edit(vrs.P(0.5, 1.5), 2.0, 2.0), step(book(left), [1, 3, 4])])
    add("Yinelenince uzaklaşan alan kolonu yine göstermez",
        [edit(vrs.P(4, 1.5), 1.0, 1.0), undo, redo, step(book(away), [1])])
    add("Geri alınınca harita eski ortasından çizer", [edit(vrs.P(4, 1.5), 6.0, 2.0), undo, step(book())])
    add("Dar görünümün alanı değişince öbür haritalar değişmez",
        [edit(vrs.P(4, 1.5), 6.0, 2.0), step(book(wide), [1, 2, 4, 7, 8])])
    half = moved(vrs.P(2, 1.5), 2.0, 2.0, math.pi)
    add("Yarım tur dönen alan: harita baş aşağı, kutu aynı", [edit(vrs.P(2, 1.5), 2.0, 2.0, math.pi), step(book(half), [1, 4])])
    three = moved(vrs.P(2, 1.5), 4.0, 0.4, 3 * math.pi / 2)
    add("Üç çeyrek dönen dar şerit", [edit(vrs.P(2, 1.5), 4.0, 0.4, 3 * math.pi / 2), step(book(three), vrs.IDS)])
    plan = {"op": "plan", "command": "cad.views.edit",
            "input": {"edit": {"kind": "area", "name": "Dar", "center": vrs.P(4, 1.5), "width": 6.0, "height": 2.0}},
            "result": vrs.ANY, "expect": {"revision": "same", "canUndo": False}}
    add("Alanın planı haritayı değiştirmez", [plan, step(book(), [4])])
    same = {"op": "execute", "command": "cad.views.edit",
            "input": {"edit": {"kind": "area", "name": "Dar", "center": vrs.P(2, 1.5), "width": 2.0, "height": 2.0}},
            "result": vrs.ANY, "expect": {"revision": "same", "canUndo": False}}
    add("Aynı alan yeniden: harita aynı, geri alma adımı yok", [same, step(book(), vrs.IDS)])
    uncropped = copy.deepcopy(wide)
    uncropped[-1]["recipe"] = {"version": 2, "level": {"id": "zemin"}, "scale": 100}
    add("Alan genişleyip kırpma kalkınca dar harita çerçevesinin bütününü sayar",
        [edit(vrs.P(4, 1.5), 6.0, 2.0),
         crop.edit("Dar", {"level": {"id": "zemin"}, "scale": 100}), step(book(uncropped), [4, 6])])
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    views = view_cases()
    sheets = sheet_cases()
    assert len(views) >= 20, len(views)
    docs = [
        (OUT_VIEWS, {"format": "kentos.command-cases", "version": 1, "command": "cad.views.edit", "commandVersion": 1,
                     "title": "Görünümün alanı (ADR 0789)",
                     "note": "scripts/fixtures/view_area_cases.py yazar; ADR 0285 ve 0789'un kuralından, KentOS kodu olmadan. "
                             "Alan değişikliği tek geri alma adımıdır; öbür değişiklikler ayar düzenlemesidir.",
                     "setup": SETUP, "cases": views}),
        (OUT_SHEETS, {"format": "kentos.command-cases", "version": 1, "command": "cad.sheets.set", "commandVersion": 1,
                      "title": "Pafta seti ve görünümün alanı (ADR 0789)",
                      "note": "scripts/fixtures/view_area_cases.py yazar; ADR 0347, 0774, 0781 ve 0789'un kurallarından, "
                              "KentOS kodu olmadan.",
                      "setup": crop.SETUP, "cases": sheets}),
    ]
    bad = 0
    for path, doc in docs:
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} bağımsız başvurudan farklı.")
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum başvuruyla aynı.")
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return bad


if __name__ == "__main__":
    sys.exit(main())
