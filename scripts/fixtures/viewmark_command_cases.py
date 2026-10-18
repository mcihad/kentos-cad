#!/usr/bin/env python3
"""The view mark and finish commands' shared cases (docs/adr/0345 §2–§4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.viewmarks.{create,edit,references}.json and cad.spaces.finish.json; `--check` compares
and writes nothing. The rules, as the ADR gives them:

- a mark's words: top its `mark`, or, empty, its sheet frame's order on its sheet (from 1); bottom its target's number:
  a named view's name, a sheet frame's sheet's name, “—” with no target; a target gone keeps the last words and breaks
  the mark (`targetGone`); a named view is known by its id, else by `v-` and its name's FNV-1a (32 bits, 8 hex digits);
- the document writes the words again in the step that adds or changes a mark, and, outside any step, whenever the named
  views or the sheet book change and after an undo or a redo (no undo step of its own);
- `cad.viewmarks.create`: no mark `no_objects`; each mark in order: its numbers finite (points x before y, `at`, radius,
  angle, size), its fields (`invalid_viewmark`: a section's 2–32 points none twice in a row, one point otherwise, a
  detail's radius over 0 and nobody else's, at most 16 letters without control characters, 0–50 mm words, a target
  either a view or a sheet with its item, no blank id), its target in the project (`target_not_found`); then the line
  weight, the revision, the layer;
- `cad.viewmarks.edit`: the ids, view marks only, `set` something, its numbers finite, an angle an elevation's, each mark's
  fields again (a new target in the project); the revision; a locked layer refusing all;
- `cad.viewmarks.references`: the marks in the drawing's order; the named views in the project's order, then the book's
  map frames in its order (“Sheet › Frame”, an unnamed frame “Çerçeve n”), each with the marks pointing at it;
- `cad.spaces.finish`: the ids, spaces only, the numbers finite, a pattern on a floor and a grid on a ceiling, spacings
  in (0, 1000] m, a scale in (0, 10 000], the pattern's name known; the revision; the layer (none given: A-ZEMN-DSN or
  A-TAVN-IZG, opened in the same step when missing); a hatch for each part of each space, tied to the space, its ring
  the part's counter-clockwise from its lowest vertex (the leftmost of the lowest), its seed the part's middle (a
  rectangle's pole), its `Kaplama` attribute the face; a floor's pattern the library's at the plot scale from the
  space's lower left corner, its name the call's, else the space's floor finish's if a pattern, else ANSI31; a ceiling's
  grid two families (0° `dy` apart, 90° `dx` apart, metres) through the space's middle or lower left corner; the space's
  earlier hatch of the face replaced (`finish_replaced`).
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as acc  # noqa: E402  (refusals, revisions, numbers as Rust writes them)
import hatch_pattern_cases as hpc  # noqa: E402  (the pattern library)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
CREATE, EDIT, REFS, FINISH = "cad.viewmarks.create", "cad.viewmarks.edit", "cad.viewmarks.references", "cad.spaces.finish"
P, refusal, num, layer, uid_of = acc.P, acc.refusal, acc.num, acc.layer, acc.uid_of
MISSING = acc.MISSING
SCALE = 50
NO_TARGET = "—"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
FORMS = {"section": "Kesit", "elevation": "Görünüş", "detail": "Detay"}
LAYERS = {"isaret": "İşaret", "mahal": "Mahal", "cizim": "Çizim", "tavan": "A-TAVN-IZG", "kilitli": "Kilitli katman",
          "gizli": "Gizli katman", "sinir": "Sınır"}


def fnv(name):
    h = 0x811C9DC5
    for b in name.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"v-{h:08x}"


VIEWS = [{"id": "v-kesit", "name": "Kesit A-A", "center": P(4, 1.5), "width": 12.0, "height": 6.0, "twist": 0.0},
         {"name": "Detay 1", "center": P(6, 1), "width": 3.0, "height": 2.0, "twist": 0.0}]
DETAY = fnv("Detay 1")


def rect(x0, y0, x1, y1):
    return [{"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}]


def mark(form, points, label, **kw):
    return {"form": form, "points": points, **kw, "words": label}


SEED = {
    1: ("space", "mahal", {"parts": rect(0, 0, 4, 3), "number": "101", "name": "Salon", "finishes": {"floor": "NET"}}),
    2: ("space", "mahal", {"parts": rect(5, 0, 8, 2), "number": "102", "name": "Mutfak", "finishes": {"floor": "Parke"}}),
    3: ("line", "cizim", {"a": P(0, 10), "b": P(5, 10)}),
    4: ("viewmark", "isaret", mark("section", [P(-1, 1.5), P(9, 1.5)], {"top": "A", "bottom": "Kesit A-A"}, mark="A",
                                   target={"view": "v-kesit"})),
    5: ("viewmark", "kilitli", mark("elevation", [P(2, -2)], {"top": "1", "bottom": "Kesit A-A"}, mark="1",
                                    target={"view": "v-kesit"})),
    6: ("viewmark", "isaret", mark("detail", [P(6, 1)], {"top": "2", "bottom": "A-101"}, radius=0.8,
                                   target={"sheet": "pafta-1", "item": "harita-2"})),
    7: ("space", "kilitli", {"parts": rect(10, 0, 12, 2), "number": "103", "name": "Depo"}),
}
IDS = sorted(SEED)
NEW = max(IDS) + 1
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Görünüm işaretleri",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": SCALE,
                 "workspace": "cad", "namedViews": VIEWS},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("isaret", "İşaret"), layer("mahal", "Mahal"), layer("cizim", "Çizim"), layer("tavan", "A-TAVN-IZG"),
               layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
               layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
    "activeLayer": "isaret",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": {}, **f} for i, (k, ly, f) in sorted(SEED.items())],
}


def map_item(item, name, left):
    return {"bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": 120000, "left": left, "top": 10000, "width": 180000}, "hidden": False, "id": item,
            "kind": {"clipToAtlasFeature": False, "grids": [], "labelBand": 0, "layers": {"type": "all"},
                     "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
                     "type": "map", "view": {"center": {"x": 4.0, "y": 1.5}, "rotation": 0, "scale": 50, "type": "fixed"}},
            "locked": False, "name": name, "opacity": 100, "padding": 0, "printable": True, "rotation": 0}


def sheet(id, name, frames):
    return {"export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "svg"}, "guides": [], "id": id,
            "items": [map_item(i, n, 20000 + 190000 * k) for k, (i, n) in enumerate(frames)], "name": name,
            "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                     "paper": "a3", "size": {"height": 297000, "width": 420000}},
            "snapGrid": {"enabled": False, "spacing": 5000, "visible": False}, "variables": [], "variants": []}


SHEETS = [("pafta-1", "A-101", [("harita-1", "Kat planı"), ("harita-2", "Kesit")])]
PRELUDE = [{"op": "execute", "command": "cad.sheets.edit", "note": "pafta kitabı: kurulum kitap veremez",
            "input": {"ops": [{"op": "addSheet", "sheet": sheet(s, n, fr)} for s, n, fr in SHEETS]},
            "result": ANY, "expect": {"revision": "changed"}}]

# ── The drawing as the steps leave it ──────────────────────────────────────────────────────────────────────────


class Drawing:
    def __init__(self):
        self.objs = {i: {"kind": k, "id": i, "layerId": ly, "attrs": {}, **copy.deepcopy(f)} for i, (k, ly, f) in SEED.items()}
        self.views = copy.deepcopy(VIEWS)
        self.sheets = copy.deepcopy(SHEETS)
        self.layers = dict(LAYERS)
        self.next = NEW

    def ids(self):
        return sorted(self.objs)

    def key(self, v):
        return v.get("id") or fnv(v["name"])

    def place(self, t):
        if "view" in t:
            for v in self.views:
                if self.key(v) == t["view"]:
                    return v["name"], None
            return None
        for s, name, frames in self.sheets:
            if s == t["sheet"]:
                for k, (i, _) in enumerate(frames):
                    if i == t["item"]:
                        return name, k + 1
        return None

    def settle(self, m):
        t = m.get("target")
        if t is None:
            m["words"] = {"top": m.get("mark", ""), "bottom": NO_TARGET}
            m.pop("broken", None)
            return
        found = self.place(t)
        if found is None:
            m["broken"] = "targetGone"
            return
        bottom, order = found
        top = m.get("mark") or (str(order) if order else "")
        m["words"] = {"top": top, "bottom": bottom}
        m.pop("broken", None)
        if not top and not bottom:
            m.pop("words")

    def refresh(self):
        for e in self.objs.values():
            if e["kind"] == "viewmark":
                self.settle(e)

    def locked(self, ly):
        return ly == "kilitli"


def entities(d, ids):
    return {str(i): copy.deepcopy(d.objs[i]) for i in ids}

# ── Checks ─────────────────────────────────────────────────────────────────────────────────────────────────────


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


def finish_writes(inp, conflict, made):
    lw = inp.get("lineWeight")
    if lw is not None and not 0 <= lw <= 100:
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"), None
    r = acc.revision(inp, conflict)
    if r:
        return r, None
    r, warn = layer_check(inp["layerId"])
    if r:
        return r, None
    return {"warnings": warn}, made


def point_bad(p, whose, path):
    return acc.point_not_finite(p, whose, path)


def not_finite(what, path):
    return refusal("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", path)


def finite_mark(n, at):
    for k, p in enumerate(n.get("points", [])):
        r = point_bad(p, "İşaretin noktasının", f"{at}.points[{k}]")
        if r:
            return r
    if "at" in n:
        r = point_bad(n["at"], "Balonun yerinin", f"{at}.at")
        if r:
            return r
    for what, field in (("Yarıçap", "radius"), ("Bakış yönü", "angle"), ("Yazı yüksekliği", "size")):
        v = n.get(field)
        if v is not None and not math.isfinite(v):
            return not_finite(what, f"{at}.{field}")
    return None


def problem(m):
    """The contract's `ViewMarkFields::problem`: the field and the words."""
    pts = m["points"]
    if m["form"] == "section":
        if not 2 <= len(pts) <= 32:
            return "points", f"Kesitin kesme çizgisi 2 ile 32 arası nokta ister, {len(pts)} verildi."
        if any(pts[k] == pts[k + 1] for k in range(len(pts) - 1)):
            return "points", "Kesme çizgisinin art arda iki noktası aynı olamaz."
    elif len(pts) != 1:
        return "points", f"{FORMS[m['form']]} tek nokta ister, {len(pts)} verildi."
    if m["form"] == "detail":
        r = m.get("radius")
        if r is None or not r > 0:
            return "radius", "Detayın sınırının yarıçapı sıfırdan büyük olmalı."
    elif m.get("radius") is not None:
        return "radius", "Yarıçap yalnız detayın sınırınındır."
    text = m.get("mark", "")
    if len(text) > 16:
        return "mark", f"{len(text)} harf; en çok 16 olur."
    if any(ord(c) < 32 or 127 <= ord(c) < 160 for c in text):
        return "mark", "Denetim karakteri olmaz."
    s = m.get("size")
    if s is not None and not 0 < s <= 50:
        return "size", f"İşaretin yazı yüksekliği {num(s)}; sıfırdan büyük, en çok 50 mm olmalı."
    t = m.get("target")
    if t is not None:
        if any(isinstance(v, str) and not v.strip() for v in t.values()):
            return "target", "Hedefin kimlikleri boş olamaz."
        if set(t) not in ({"view"}, {"sheet", "item"}):
            return "target", "Hedef ya bir adlı görünümdür (view) ya da bir paftadaki görünüm penceresi (sheet ve item)."
    return None


def gone(t, path):
    if "view" in t:
        words = f"“{t['view']}” kimlikli adlı görünüm projede yok. Görünümler penceresindeki bir görünümün kimliğini verin."
    else:
        words = f"“{t['sheet']}” paftasında “{t['item']}” kimlikli görünüm penceresi yok. Pafta kitabındaki bir harita çerçevesini verin."
    return refusal("target_not_found", words, path)


def fields_of(n):
    m = {"form": n["form"], "points": n["points"]}
    for k in ("radius", "at", "angle"):
        if n.get(k) is not None:
            m[k] = n[k]
    if n.get("flip"):
        m["flip"] = True
    if n.get("mark"):
        m["mark"] = n["mark"]
    if n.get("target") is not None:
        m["target"] = n["target"]
    if n.get("size") is not None:
        m["size"] = n["size"]
    return m


def extra_of(inp):
    return {k: inp[k] for k in ("color", "lineWeight") if k in inp}


def create_check(d, inp, conflict=False):
    if not inp["marks"]:
        return refusal("no_objects", "Görünüm işareti verilmedi. En az bir işaret (marks) verin.", "marks"), None
    made = []
    for i, n in enumerate(inp["marks"]):
        at = f"marks[{i}]"
        r = finite_mark(n, at)
        if r:
            return r, None
        m = fields_of(n)
        pr = problem(m)
        if pr:
            return refusal("invalid_viewmark", pr[1], f"{at}.{pr[0]}"), None
        if m.get("target") is not None and d.place(m["target"]) is None:
            return gone(m["target"], f"{at}.target"), None
        d.settle(m)
        made.append(m)
    return finish_writes(inp, conflict, made)


def create_step(d, op, inp, non_finite=None, conflict=False):
    result, made = create_check(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": CREATE, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if made is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    warnings = result["warnings"]
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": warnings}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [{"kind": "viewmark", "id": 0, "layerId": inp["layerId"], "attrs": {}, **extra_of(inp), **m} for m in made]
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"marks": ents, "revision": "$current"}, "warnings": warnings}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, e in zip(ids, ents):
        d.objs[i] = {**e, "id": i}
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": warnings}
    s["expect"] = {"ids": d.ids(), "entities": entities(d, ids), "canUndo": True, "dirty": True, "revision": "changed"}
    return s


def is_uid(u):
    return acc.is_uid(u) or u.startswith("$uidOf:")


def objects_check(d, uids, nothing):
    if not uids:
        return refusal("no_entities", f"{nothing} En az bir nesnenin kalıcı kimliğini verin.", "uids"), None
    for i, u in enumerate(uids):
        if not is_uid(u):
            return refusal("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", f"uids[{i}]"), None
    found, seen = [], set()
    for i, u in enumerate(uids):
        if u in seen:
            continue
        seen.add(u)
        oid = int(u.split(":")[1]) if u.startswith("$uidOf:") else None
        if oid not in d.objs:
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", f"uids[{i}]"), None
        found.append((i, oid))
    return None, found


def edit_check(d, inp, conflict=False):
    r, found = objects_check(d, inp["uids"], "Düzenlenecek görünüm işareti verilmedi.")
    if r:
        return r, None
    for at, oid in found:
        if d.objs[oid]["kind"] != "viewmark":
            return refusal("not_a_viewmark", "Nesne görünüm işareti değil; Görünüm işaretini düzenle yalnız kesit, görünüş ve detay işaretlerini değiştirir.", f"uids[{at}]"), None
    op = inp["operation"]
    kind = op["kind"]
    if kind == "set":
        keys = [k for k in ("mark", "size", "flip", "angle", "radius") if k in op]
        if not keys:
            return refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi.", "operation"), None
        for what, field in (("Yazı yüksekliği", "size"), ("Bakış yönü", "angle"), ("Yarıçap", "radius")):
            v = op.get(field)
            if v is not None and not math.isfinite(v):
                return not_finite(what, f"operation.{field}"), None
    out = []
    for at, oid in found:
        m = copy.deepcopy(d.objs[oid])
        if kind == "set":
            if "mark" in op:
                if op["mark"]:
                    m["mark"] = op["mark"]
                else:
                    m.pop("mark", None)
            if "size" in op:
                if op["size"] is None:
                    m.pop("size", None)
                else:
                    m["size"] = op["size"]
            if "flip" in op:
                if op["flip"]:
                    m["flip"] = True
                else:
                    m.pop("flip", None)
            if "angle" in op:
                if m["form"] != "elevation":
                    return refusal("invalid_viewmark", "Bakış yönü yalnız görünüş işaretinindir.", "operation.angle"), None
                m["angle"] = op["angle"]
            if "radius" in op:
                m["radius"] = op["radius"]
        elif kind == "retarget":
            m["target"] = op["target"]
        else:
            m.pop("target", None)
        pr = problem(m)
        if pr:
            field = pr[0]
            if kind == "set" and field in ("mark", "size", "angle", "radius"):
                path = f"operation.{field}"
            elif kind == "retarget" and field == "target":
                path = "operation.target"
            else:
                path = f"uids[{at}]"
            return refusal("invalid_viewmark", pr[1], path), None
        if kind == "retarget" and d.place(m["target"]) is None:
            return gone(m["target"], "operation.target"), None
        d.settle(m)
        out.append((oid, m))
    r = acc.revision(inp, conflict)
    if r:
        return r, None
    for at, oid in found:
        ly = d.objs[oid]["layerId"]
        if d.locked(ly):
            return refusal("layer_locked", f"“{LAYERS[ly]}” katmanı kilitli; üzerindeki işaret düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{at}]"), None
    return None, out


def edit_step(d, op, inp, non_finite=None, conflict=False):
    r, out = edit_check(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": EDIT, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if r:
        s["result"] = r
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    marks = [copy.deepcopy(m) for _, m in out]
    s["result"] = {"status": "completed", "output": {"marks": marks, "revision": "$current"}, "warnings": []}
    if op == "plan":
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    for oid, m in out:
        d.objs[oid] = m
    s["expect"] = {"ids": d.ids(), "entities": entities(d, [o for o, _ in out]), "canUndo": True, "revision": "changed"}
    return s


def references_output(d):
    marks = []
    for i in d.ids():
        e = d.objs[i]
        if e["kind"] != "viewmark":
            continue
        m = {"uid": uid_of(i), "form": e["form"]}
        if e.get("mark"):
            m["mark"] = e["mark"]
        if e.get("target") is not None:
            m["target"] = e["target"]
        m["words"] = e.get("words", {"top": "", "bottom": ""})
        m["broken"] = "broken" in e
        m["at"] = e["points"][0]
        marks.append(m)

    def pointing(t):
        return [m["uid"] for m in marks if m.get("target") == t]

    targets = []
    for v in d.views:
        t = {"view": d.key(v)}
        targets.append({"target": t, "name": v["name"], "number": v["name"], "marks": pointing(t)})
    for s, name, frames in d.sheets:
        for k, (i, frame) in enumerate(frames):
            t = {"sheet": s, "item": i}
            targets.append({"target": t, "name": f"{name} › {frame or f'Çerçeve {k + 1}'}", "number": name, "order": k + 1,
                            "marks": pointing(t)})
    return {"marks": marks, "targets": targets, "revision": "$current"}


def references_step(d, op="execute"):
    out = references_output(d)
    return {"op": op, "command": REFS, "input": {}, "result": {"status": "completed", "output": None if op == "validate" else out, "warnings": []},
            "expect": {"ids": d.ids(), "revision": "same"}}


def refreshed(d, step, entity_ids=None):
    """A step outside the marks' commands after which the document writes their words again in place."""
    d.refresh()
    marks = entity_ids or [i for i in d.ids() if d.objs[i]["kind"] == "viewmark"]
    step.setdefault("expect", {})["entities"] = entities(d, marks)
    return step


def rename_view(d, name, to):
    for v in d.views:
        if v["name"] == name:
            if "id" not in v:
                v["id"] = fnv(name)
            v["name"] = to
    return refreshed(d, {"op": "execute", "command": "cad.views.edit", "input": {"edit": {"kind": "rename", "name": name, "to": to}},
                         "result": ANY, "expect": {"revision": "changed"}})


def delete_view(d, name):
    d.views = [v for v in d.views if v["name"] != name]
    return refreshed(d, {"op": "execute", "command": "cad.views.edit", "input": {"edit": {"kind": "delete", "name": name}},
                         "result": ANY, "expect": {"revision": "changed"}})


def save_view(d, name):
    v = {"name": name, "center": P(1, 1), "width": 4.0, "height": 3.0, "twist": 0.0}
    old = next((x for x in d.views if x["name"] == name), None)
    if old:
        v = {**old, **v}
        d.views = [v if x is old else x for x in d.views]
    else:
        d.views.append(v)
    return refreshed(d, {"op": "execute", "command": "cad.views.edit",
                         "input": {"edit": {"kind": "save", "name": name, "center": P(1, 1), "width": 4.0, "height": 3.0}},
                         "result": ANY, "expect": {"revision": "changed"}})


def rename_sheet(d, sid, to):
    d.sheets = [(s, to if s == sid else n, fr) for s, n, fr in d.sheets]
    return refreshed(d, {"op": "execute", "command": "cad.sheets.edit", "input": {"ops": [{"op": "renameSheet", "id": sid, "name": to}]},
                         "result": ANY, "expect": {"revision": "changed"}})


def remove_sheet(d, sid):
    d.sheets = [x for x in d.sheets if x[0] != sid]
    return refreshed(d, {"op": "execute", "command": "cad.sheets.edit", "input": {"ops": [{"op": "removeSheet", "id": sid}]},
                         "result": ANY, "expect": {"revision": "changed"}})


def add_sheet(d, sid, name, frames, index=None):
    op = {"op": "addSheet", "sheet": sheet(sid, name, frames)}
    if index is not None:
        op["index"] = index
        d.sheets.insert(index, (sid, name, frames))
    else:
        d.sheets.append((sid, name, frames))
    return refreshed(d, {"op": "execute", "command": "cad.sheets.edit", "input": {"ops": [op]}, "result": ANY,
                         "expect": {"revision": "changed"}})


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        assert len(name) > 10, name
        assert name not in [c["name"] for c in self.cases], name
        self.cases.append({"name": name, "steps": copy.deepcopy(PRELUDE) + steps})


def undo_step(d_before, label, ids=None, entity_ids=()):
    s = {"op": "undo", "returns": label, "expect": {"ids": ids if ids is not None else d_before.ids(), "canRedo": True, "revision": "changed"}}
    if entity_ids:
        s["expect"]["entities"] = entities(d_before, entity_ids)
    return s

# ── cad.viewmarks.create ───────────────────────────────────────────────────────────────────────────────────────


SECTION = {"form": "section", "points": [P(0, 4), P(8, 4)], "mark": "B", "target": {"sheet": "pafta-1", "item": "harita-2"}}
ONE = {"layerId": "isaret", "marks": [SECTION]}


def create_cases():
    c = Cases()
    c.add("Kesit pafta penceresine: alt yarıda paftanın adı", [create_step(Drawing(), "execute", ONE)])
    c.add("Doğrula yazmaz", [create_step(Drawing(), "validate", ONE)])
    c.add("Plan yazmadan gösterir, yazısı hedeften", [create_step(Drawing(), "plan", ONE)])
    d = Drawing()
    step = create_step(d, "execute", ONE)
    c.add("Yazılır ve geri alınır", [step, undo_step(Drawing(), "Görünüm işareti", IDS)])
    for name, n in [
        ("Numarasız kesit: üst yarıya pencerenin paftadaki sırası", {**SECTION, "mark": ""}),
        ("Adlı görünüme bakan görünüş: alt yarıda görünümün adı", {"form": "elevation", "points": [P(3, -1)], "angle": 0.0, "mark": "2",
                                                                  "target": {"view": "v-kesit"}}),
        ("Kimliksiz görünüm adından türeyen kimlikle bulunur", {"form": "elevation", "points": [P(3, -1)], "mark": "3", "target": {"view": DETAY}}),
        ("Hedefsiz detay: alt yarıda tire", {"form": "detail", "points": [P(2, 2)], "radius": 0.5, "mark": "D1"}),
        ("Detay balonun yeriyle ve 3,5 mm yazıyla", {"form": "detail", "points": [P(2, 2)], "radius": 0.5, "at": P(4, 4), "size": 3.5,
                                                     "target": {"sheet": "pafta-1", "item": "harita-1"}}),
        ("Kırık kesme çizgisi, bakış çevrilmiş", {"form": "section", "points": [P(0, 0), P(3, 0), P(3, 3), P(6, 3)], "flip": True, "mark": "C"}),
        ("Numarasız ve hedefsiz: üst yarı boş, alt yarıda tire", {"form": "elevation", "points": [P(0, 0)]}),
    ]:
        c.add(name, [create_step(Drawing(), "execute", {"layerId": "isaret", "marks": [n]})])
    c.add("İki işaret tek adımda, sırayla", [create_step(Drawing(), "execute", {"layerId": "isaret", "marks": [
        SECTION, {"form": "elevation", "points": [P(9, 9)], "angle": 180.0, "mark": "4"}]})])
    c.add("Renk ve çizgi kalınlığıyla", [create_step(Drawing(), "execute", {**ONE, "color": "#ff0000", "lineWeight": 0.35})])
    c.add("Gizli katmana uyarıyla yazılır", [create_step(Drawing(), "execute", {**ONE, "layerId": "gizli"})])
    refused = [
        ("İşaret verilmezse reddedilir", {"layerId": "isaret", "marks": []}, {}),
        ("Kesitin tek noktası reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "points": [P(0, 0)]}]}, {}),
        ("Kesitin art arda aynı noktaları reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "points": [P(0, 0), P(0, 0), P(1, 1)]}]}, {}),
        ("Görünüşün iki noktası reddedilir", {"layerId": "isaret", "marks": [{"form": "elevation", "points": [P(0, 0), P(1, 0)]}]}, {}),
        ("Yarıçapsız detay reddedilir", {"layerId": "isaret", "marks": [{"form": "detail", "points": [P(0, 0)]}]}, {}),
        ("Sıfır yarıçaplı detay reddedilir", {"layerId": "isaret", "marks": [{"form": "detail", "points": [P(0, 0)], "radius": 0.0}]}, {}),
        ("Kesitin yarıçapı reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "radius": 1.0}]}, {}),
        ("On yedi harfli numara reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "mark": "ABCDEFGHIJKLMNOPQ"}]}, {}),
        ("Denetim karakterli numara reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "mark": "A\tB"}]}, {}),
        ("Altmış milimetrelik yazı reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "size": 60.0}]}, {}),
        ("Görünüm ve pafta birlikte hedef olmaz", {"layerId": "isaret", "marks": [{**SECTION, "target": {"view": "v-kesit", "sheet": "pafta-1", "item": "harita-1"}}]}, {}),
        ("Boş görünüm kimliği reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "target": {"view": " "}}]}, {}),
        ("Olmayan görünüm reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "target": {"view": "v-yok"}}]}, {}),
        ("Olmayan pafta penceresi reddedilir", {"layerId": "isaret", "marks": [{**SECTION, "target": {"sheet": "pafta-1", "item": "harita-9"}}]}, {}),
        ("İkinci işaretin hatası yolunu söyler", {"layerId": "isaret", "marks": [SECTION, {"form": "elevation", "points": []}]}, {}),
        ("Sonlu olmayan nokta reddedilir", ONE, {"non_finite": {"marks[0].points[1].y": "NaN"}}),
        ("Sonlu olmayan yarıçap reddedilir", {"layerId": "isaret", "marks": [{"form": "detail", "points": [P(0, 0)], "radius": 1.0}]},
         {"non_finite": {"marks[0].radius": "Infinity"}}),
        ("Olmayan katman reddedilir", {**ONE, "layerId": "yok"}, {}),
        ("Katman grubu reddedilir", {**ONE, "layerId": "pafta"}, {}),
        ("Kilitli katman reddedilir", {**ONE, "layerId": "kilitli"}, {}),
        ("Geçersiz çizgi kalınlığı reddedilir", {**ONE, "lineWeight": 120.0}, {}),
        ("Geçersiz beklenen sürüm reddedilir", {**ONE, "expectedRevision": "07"}, {}),
    ]
    for name, inp, kw in refused:
        c.add(name, [create_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = create_step(d, "execute", ONE)
    stale = create_step(d, "execute", {**ONE, "expectedRevision": "$prepared"}, conflict=True)
    c.add("Sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c

# ── cad.viewmarks.edit ─────────────────────────────────────────────────────────────────────────────────────────


def ed(uids, operation, **kw):
    return {"uids": [uid_of(u) for u in uids], "operation": operation, **kw}


def edit_cases():
    c = Cases()
    for name, inp in [
        ("Numarayı değiştir: üst yarı yeni numara", ed([4], {"kind": "set", "mark": "B"})),
        ("Numarayı sil: görünüme bakan işaretin üst yarısı boş kalır", ed([4], {"kind": "set", "mark": ""})),
        ("Bakışı çevir", ed([4], {"kind": "set", "flip": True})),
        ("Yazı yüksekliği ve varsayılana dönüş", ed([4], {"kind": "set", "size": 5.0})),
        ("Detayın yarıçapı", ed([6], {"kind": "set", "radius": 1.25})),
        ("Pafta penceresine yeniden bağla", ed([4], {"kind": "retarget", "target": {"sheet": "pafta-1", "item": "harita-1"}})),
        ("Görünüme yeniden bağla, kimliği addan", ed([6], {"kind": "retarget", "target": {"view": DETAY}})),
        ("Bağı kopar: alt yarıda tire", ed([4], {"kind": "detach"})),
        ("İki işaret birlikte kopar", ed([4, 6], {"kind": "detach"})),
    ]:
        d = Drawing()
        c.add(name, [edit_step(d, "execute", inp), undo_step(Drawing(), "Görünüm işaretini düzenle", IDS, [4, 6])])
    c.add("Doğrula yazmaz", [edit_step(Drawing(), "validate", ed([4], {"kind": "set", "mark": "Z"}))])
    c.add("Plan yazmadan gösterir", [edit_step(Drawing(), "plan", ed([4], {"kind": "detach"}))])
    d = Drawing()
    c.add("Hedefi silinmiş işaretin numarası değişir, kopukluğu kalır",
          [delete_view(d, "Kesit A-A"), edit_step(d, "execute", ed([4], {"kind": "set", "mark": "K"}))])
    d = Drawing()
    c.add("Kopuk işaret yeniden bağlanınca onarılır",
          [delete_view(d, "Kesit A-A"), edit_step(d, "execute", ed([4], {"kind": "retarget", "target": {"view": DETAY}}))])
    refused = [
        ("Kimlik verilmezse reddedilir", {"uids": [], "operation": {"kind": "detach"}}, {}),
        ("Geçersiz kimlik reddedilir", {"uids": ["kimlik"], "operation": {"kind": "detach"}}, {}),
        ("Olmayan nesne reddedilir", {"uids": [MISSING], "operation": {"kind": "detach"}}, {}),
        ("İşaret olmayan nesne reddedilir", ed([4, 3], {"kind": "detach"}), {}),
        ("Boş değişiklik reddedilir", ed([4], {"kind": "set"}), {}),
        ("Kesite bakış yönü verilemez", ed([4], {"kind": "set", "angle": 45.0}), {}),
        ("Kesite yarıçap verilemez", ed([4], {"kind": "set", "radius": 1.0}), {}),
        ("Uzun numara reddedilir", ed([4], {"kind": "set", "mark": "X" * 17}), {}),
        ("Sıfır yazı yüksekliği reddedilir", ed([4], {"kind": "set", "size": 0.0}), {}),
        ("Olmayan pencereye bağlanmaz", ed([4], {"kind": "retarget", "target": {"sheet": "pafta-9", "item": "harita-1"}}), {}),
        ("Eksik pafta hedefi reddedilir", ed([4], {"kind": "retarget", "target": {"sheet": "pafta-1"}}), {}),
        ("Sonlu olmayan yazı yüksekliği reddedilir", ed([4], {"kind": "set", "size": 1.0}), {"non_finite": {"operation.size": "NaN"}}),
        ("Kilitli katmandaki işaret hepsini reddeder", ed([4, 5], {"kind": "set", "mark": "Q"}), {}),
        ("Geçersiz beklenen sürüm reddedilir", ed([4], {"kind": "detach"}, expectedRevision="x"), {}),
    ]
    for name, inp, kw in refused:
        c.add(name, [edit_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = edit_step(d, "execute", ed([4], {"kind": "set", "mark": "M"}))
    stale = edit_step(d, "execute", ed([4], {"kind": "detach"}, expectedRevision="$prepared"), conflict=True)
    c.add("Sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c

# ── cad.viewmarks.references and the following ─────────────────────────────────────────────────────────────────


def reference_cases():
    c = Cases()
    c.add("Kurulumun işaretleri ve hedefleri", [references_step(Drawing())])
    c.add("Plan: aynı cevap", [references_step(Drawing(), "plan")])
    c.add("Doğrula: yazmaz", [references_step(Drawing(), "validate")])
    d = Drawing()
    c.add("Görünüm yeniden adlandırılınca işaret aynı anda yeni adı yazar, geri alma adımı olmadan",
          [rename_view(d, "Kesit A-A", "Kesit B-B"), references_step(d)])
    d = Drawing()
    c.add("Kimliksiz görünüm yeniden adlandırılınca eski adının kimliğini alır",
          [rename_view(d, "Detay 1", "Mutfak detayı"), references_step(d)])
    d = Drawing()
    s = edit_step(d, "execute", ed([4], {"kind": "retarget", "target": {"view": DETAY}}))
    c.add("Kimliği addan gelen görünüme bağlı işaret görünüm yeniden adlandırılınca izler",
          [s, rename_view(d, "Detay 1", "Detay 2"), references_step(d)])
    d = Drawing()
    c.add("Görünüm silinince işaret son yazısıyla kopar", [delete_view(d, "Kesit A-A"), references_step(d)])
    d = Drawing()
    made = create_step(d, "execute", {"layerId": "isaret", "marks": [
        {"form": "elevation", "points": [P(0, 0)], "mark": "9", "target": {"view": DETAY}}]})
    c.add("Silinen görünüm aynı adla yeniden kaydedilince işaret onarılır",
          [made, delete_view(d, "Detay 1"), save_view(d, "Detay 1"), references_step(d)])
    d = Drawing()
    c.add("Pafta yeniden adlandırılınca alt yarı yeni adı yazar", [rename_sheet(d, "pafta-1", "A-102"), references_step(d)])
    d = Drawing()
    c.add("Pafta silinince ona bakan işaret kopar", [remove_sheet(d, "pafta-1"), references_step(d)])
    d = Drawing()
    c.add("Kopan işaretin paftası geri gelince onarılır",
          [remove_sheet(d, "pafta-1"), add_sheet(d, "pafta-1", "A-101", SHEETS[0][2]), references_step(d)])
    d = Drawing()
    c.add("Pencerenin sırası değişince numarasız işaretin üst yarısı izler",
          [remove_sheet(d, "pafta-1"), add_sheet(d, "pafta-1", "A-101", [("harita-2", "Kesit"), ("harita-1", "Kat planı")]),
           references_step(d)])
    d = Drawing()
    c.add("İkinci pafta listenin sonunda, penceresi sırasıyla",
          [add_sheet(d, "pafta-2", "A-201", [("harita-3", "Görünüş"), ("harita-4", "Detaylar")]), references_step(d)])
    d = Drawing()
    c.add("Yeni işaret listede çizimin sırasıyla", [create_step(d, "execute", ONE), references_step(d)])
    d = Drawing()
    before = Drawing()
    c.add("Geri alınan işaret listeden çıkar", [create_step(d, "execute", ONE), undo_step(before, "Görünüm işareti", IDS),
                                                references_step(before)])
    d = Drawing()
    s = edit_step(d, "execute", ed([4], {"kind": "detach"}))
    c.add("Bağı kopan işaret hedefin listesinden çıkar", [s, references_step(d)])
    d = Drawing()
    rn = rename_view(d, "Kesit A-A", "Kesit C-C")
    edit = edit_step(d, "execute", ed([4], {"kind": "set", "mark": "C"}))
    after_undo = copy.deepcopy(d)
    after_undo.objs[4]["mark"] = "A"
    after_undo.refresh()
    c.add("Geri alma işaretin yazısını hedefin bugünkü adıyla yeniden yazar",
          [rn, edit, {"op": "undo", "returns": "Görünüm işaretini düzenle", "expect": {"entities": entities(after_undo, [4])}},
           references_step(after_undo)])
    d = Drawing()
    rn = rename_view(d, "Kesit A-A", "Kesit D-D")
    c.add("Yeniden adlandırma geri alma adımı değildir", [rn, {"op": "undo", "returns": None, "expect": {"entities": entities(d, [4])}}])
    d = Drawing()
    c.add("Başa eklenen pafta işaretlerin sırasını değiştirmez", [add_sheet(d, "pafta-0", "A-001", [("harita-0", "Kapak")], index=0),
                                                                 references_step(d)])
    d = Drawing()
    c.add("Kilitli katmandaki işaret de hedefini izler", [rename_view(d, "Kesit A-A", "Kesit E-E"),
                                                         {"op": "execute", "command": EDIT, "input": ed([5], {"kind": "detach"}),
                                                          "result": refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki işaret düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"),
                                                          "expect": {"entities": entities(d, [5]), "revision": "same"}}])
    d = Drawing()
    s = create_step(d, "execute", {"layerId": "isaret", "marks": [{"form": "section", "points": [P(0, 0), P(1, 0)], "target": {"view": "v-kesit"}}]})
    c.add("Görünüme bakan numarasız işaretin üst yarısı boş", [s, references_step(d)])
    return c

# ── cad.spaces.finish ──────────────────────────────────────────────────────────────────────────────────────────


def ccw_from_lowest(pts):
    area = sum(pts[i]["x"] * pts[(i + 1) % len(pts)]["y"] - pts[(i + 1) % len(pts)]["x"] * pts[i]["y"] for i in range(len(pts)))
    ring = pts if area > 0 else list(reversed(pts))
    k = min(range(len(ring)), key=lambda i: (ring[i]["y"], ring[i]["x"]))
    return ring[k:] + ring[:k]


def box(e):
    pts = [p for part in e["parts"] for p in part["outer"]["pts"]]
    return (min(p["x"] for p in pts), min(p["y"] for p in pts)), (max(p["x"] for p in pts), max(p["y"] for p in pts))


def library_pattern(name, scale):
    lib = hpc.LIB[name]
    return {"type": "pattern", "angle": 0, "spacing": 1, "name": lib["name"], "scale": scale * SCALE / 1000.0, "lines": lib["lines"]}


def known(name):
    return name.upper() in hpc.LIB


def turn(a):
    t = math.fmod(a, 360.0)
    return t + 360.0 if t < 0 else t


def finish_check(d, inp, conflict=False):
    r, found = objects_check(d, inp["uids"], "Kaplanacak mahal verilmedi.")
    if r:
        return r, None
    for at, oid in found:
        if d.objs[oid]["kind"] != "space":
            return refusal("not_a_space", "Nesne mahal değil; döşeme deseni ve tavan ızgarası mahallere yazılır.", f"uids[{at}]"), None
    if inp.get("scale") is not None and not math.isfinite(inp["scale"]):
        return refusal("not_finite", "Desenin ölçeği sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "scale"), None
    g = inp.get("grid")
    if g:
        for f in ("dx", "dy", "angle"):
            if not math.isfinite(g.get(f, 0.0)):
                return refusal("not_finite", "Izgaranın değeri sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", f"grid.{f}"), None
    bad = lambda w, p: (refusal("invalid_finish", w, p), None)  # noqa: E731
    if inp["surface"] == "floor" and g:
        return bad("Döşemeye ızgara değil desen yazılır; tavan için surface: ceiling verin.", "grid")
    if inp["surface"] == "ceiling" and ("pattern" in inp or "scale" in inp):
        return bad("Tavana desen değil ızgara yazılır; aralıkları grid ile verin.", "pattern" if "pattern" in inp else "scale")
    if inp["surface"] == "ceiling" and not g:
        return bad("Tavan ızgarasının aralıkları verilmedi; grid ile dx ve dy verin.", "grid")
    if g:
        for f in ("dx", "dy"):
            if not 0 < g[f] <= 1000:
                return bad("Izgaranın aralığı sıfırdan büyük, en çok 1000 m olmalı.", f"grid.{f}")
    if inp.get("scale") is not None and not 0 < inp["scale"] <= 10000:
        return bad("Desenin ölçeği sıfırdan büyük, en çok 10 000 olmalı.", "scale")
    if inp.get("pattern") is not None and not known(inp["pattern"]):
        return refusal("unknown_pattern", f"“{inp['pattern']}” adında desen yok. Desen kitaplığındaki ya da projenin desenlerindeki bir adı verin (ANSI31, NET, BRICK …).", "pattern"), None
    r = acc.revision(inp, conflict)
    if r:
        return r, None
    face = "Döşeme" if inp["surface"] == "floor" else "Tavan"
    warnings = []
    opened = None
    if "layerId" in inp:
        r, warnings = layer_check(inp["layerId"])
        if r:
            return r, None
        target = inp["layerId"]
    elif inp["surface"] == "ceiling":
        target = "tavan"
    else:
        target, opened = "a-zemn-dsn", "A-ZEMN-DSN"
    hatches, replaced = [], []
    for at, oid in found:
        sp = d.objs[oid]
        for hid, h in sorted(d.objs.items()):
            if h["kind"] == "hatch" and h.get("assoc", {}).get("outer") == uid_of(oid) and h["attrs"].get("Kaplama") == face:
                replaced.append(hid)
        lo, hi = box(sp)
        if g:
            origin = P((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2) if g.get("origin", "center") == "center" else P(*lo)
            pat = {"type": "pattern", "angle": turn(g.get("angle", 0.0)), "spacing": 1, "name": "TAVAN", "scale": 1,
                   "lines": [{"angle": 0, "origin": [0, 0], "offset": [0, g["dy"]]}, {"angle": 90, "origin": [0, 0], "offset": [0, g["dx"]]}],
                   "origin": origin}
        else:
            own = sp.get("finishes", {}).get("floor")
            name = inp.get("pattern") or (own if own and known(own) else "ANSI31")
            pat = {**library_pattern(name.upper(), inp.get("scale", 1.0)), "origin": P(*lo)}
        for part in sp["parts"]:
            pts = part["outer"]["pts"]
            seed = P((min(p["x"] for p in pts) + max(p["x"] for p in pts)) / 2, (min(p["y"] for p in pts) + max(p["y"] for p in pts)) / 2)
            hatches.append({"kind": "hatch", "layerId": target, "attrs": {"Kaplama": face}, "ring": ccw_from_lowest(pts), "pattern": pat,
                            "assoc": {"outer": uid_of(oid), "seed": seed}})
    if replaced:
        warnings = warnings + [{"code": "finish_replaced", "message": f"{len(replaced)} mahalin eski {face.lower()} kaplaması kaldırıldı.", "path": "uids"}]
    return {"warnings": warnings, "opened": opened, "target": target}, (hatches, replaced)


def finish_step(d, op, inp, non_finite=None, conflict=False):
    r, made = finish_check(d, acc.with_non_finite(inp, non_finite or {}), conflict)
    s = {"op": op, "command": FINISH, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if made is None:
        s["result"] = r
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    hatches, replaced = made
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": r["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"hatches": [{**h, "id": 0} for h in hatches],
                                                         "replaced": [uid_of(i) for i in replaced], "revision": "$current"},
                       "warnings": r["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    # A replaced hatch's slot is empty afterwards: its id as a step before took it.
    gone_uids = [f"$uid:h{i}" for i in replaced]
    for i in replaced:
        del d.objs[i]
    ids = list(range(d.next, d.next + len(hatches)))
    d.next += len(hatches)
    for i, h in zip(ids, hatches):
        d.objs[i] = {**h, "id": i}
    s["result"] = {"status": "completed", "output": {"hatches": [copy.deepcopy(d.objs[i]) for i in ids], "replaced": gone_uids, "revision": "$current"},
                   "warnings": r["warnings"]}
    s["expect"] = {"ids": d.ids(), "entities": entities(d, ids), "canUndo": True, "dirty": True, "revision": "changed"}
    return s


FLOOR = {"uids": [uid_of(2)], "surface": "floor"}
CEILING = {"uids": [uid_of(1)], "surface": "ceiling", "grid": {"dx": 0.6, "dy": 0.6}}


def finish_cases():
    c = Cases()
    for name, inp in [
        ("Döşeme deseni: kaplamanın adı desen değilse ANSI31, katmanı aynı adımda açılır", FLOOR),
        ("Döşeme deseni: mahalin döşeme kaplaması NET ise NET", {"uids": [uid_of(1)], "surface": "floor"}),
        ("Döşeme deseni adıyla ve ölçeğiyle", {**FLOOR, "pattern": "brick", "scale": 2.0}),
        ("İki mahal tek adımda", {"uids": [uid_of(1), uid_of(2)], "surface": "floor", "pattern": "NET"}),
        ("Verilen katmana", {**FLOOR, "layerId": "cizim"}),
        ("Tavan ızgarası 60 × 60: katmanı adıyla bulunur, ortadan", CEILING),
        ("Tavan ızgarası köşeden ve dönük", {**CEILING, "grid": {"dx": 0.6, "dy": 1.2, "angle": -15.0, "origin": "corner"}}),
        ("Kilitli katmandaki mahal kaplanır: tarama başka katmanda", {"uids": [uid_of(7)], "surface": "floor"}),
    ]:
        d = Drawing()
        c.add(name, [finish_step(d, "execute", inp), undo_step(Drawing(), "Döşeme ve tavan", IDS)])
    c.add("Doğrula yazmaz", [finish_step(Drawing(), "validate", FLOOR)])
    c.add("Plan yazmadan gösterir", [finish_step(Drawing(), "plan", CEILING)])
    c.add("Gizli katmana uyarıyla", [finish_step(Drawing(), "execute", {**FLOOR, "layerId": "gizli"})])
    d = Drawing()
    first = finish_step(d, "execute", FLOOR)
    second = finish_step(d, "execute", {**FLOOR, "pattern": "NET"})
    c.add("Aynı yüzün eski taraması yerine geçer", [first, {"op": "captureUid", "as": "h8", "id": 8}, second])
    d = Drawing()
    first = finish_step(d, "execute", {"uids": [uid_of(1)], "surface": "floor"})
    second = finish_step(d, "execute", {**CEILING})
    c.add("Döşeme ve tavan birbirinin yerine geçmez", [first, second])
    d = Drawing()
    plan_after = finish_step(d, "execute", FLOOR)
    c.add("Planda yerine geçecek tarama söylenir", [plan_after, finish_step(d, "plan", {**FLOOR, "pattern": "LINE"})])
    refused = [
        ("Mahal verilmezse reddedilir", {"uids": [], "surface": "floor"}, {}),
        ("Geçersiz kimlik reddedilir", {"uids": ["mahal"], "surface": "floor"}, {}),
        ("Olmayan mahal reddedilir", {"uids": [MISSING], "surface": "floor"}, {}),
        ("Mahal olmayan nesne reddedilir", {"uids": [uid_of(3)], "surface": "floor"}, {}),
        ("Döşemeye ızgara reddedilir", {**FLOOR, "grid": {"dx": 1.0, "dy": 1.0}}, {}),
        ("Tavana desen reddedilir", {**CEILING, "pattern": "NET"}, {}),
        ("Tavana ölçek reddedilir", {**CEILING, "scale": 2.0}, {}),
        ("Izgarasız tavan reddedilir", {"uids": [uid_of(1)], "surface": "ceiling"}, {}),
        ("Sıfır aralıklı ızgara reddedilir", {**CEILING, "grid": {"dx": 0.0, "dy": 0.6}}, {}),
        ("Sıfır ölçek reddedilir", {**FLOOR, "scale": 0.0}, {}),
        ("Bilinmeyen desen reddedilir", {**FLOOR, "pattern": "Mermer"}, {}),
        ("Sonlu olmayan ölçek reddedilir", {**FLOOR, "scale": 1.0}, {"non_finite": {"scale": "NaN"}}),
        ("Sonlu olmayan ızgara açısı reddedilir", {**CEILING, "grid": {"dx": 0.6, "dy": 0.6, "angle": 0.0}}, {"non_finite": {"grid.angle": "Infinity"}}),
        ("Olmayan katman reddedilir", {**FLOOR, "layerId": "yok"}, {}),
        ("Kilitli katman reddedilir", {**FLOOR, "layerId": "kilitli"}, {}),
        ("Katman grubu reddedilir", {**FLOOR, "layerId": "pafta"}, {}),
        ("Geçersiz beklenen sürüm reddedilir", {**FLOOR, "expectedRevision": "07"}, {}),
    ]
    for name, inp, kw in refused:
        c.add(name, [finish_step(Drawing(), "execute", inp, **kw)])
    d = Drawing()
    made = finish_step(d, "execute", FLOOR)
    stale = finish_step(d, "execute", {**CEILING, "expectedRevision": "$prepared"}, conflict=True)
    c.add("Sürüm değişince yazılmaz (çakışma)", [{"op": "captureRevision", "as": "prepared"}, made, stale])
    return c


def doc(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/viewmark_command_cases.py yazar; ADR 0345'in kurallarından, KentOS kodu olmadan.",
            "setup": SETUP, "cases": cases.cases}


def files():
    return {
        f"{CREATE}.json": doc(CREATE, "Görünüm işareti", create_cases()),
        f"{EDIT}.json": doc(EDIT, "Görünüm işaretini düzenle", edit_cases()),
        f"{REFS}.json": doc(REFS, "Başvurular", reference_cases()),
        f"{FINISH}.json": doc(FINISH, "Döşeme ve tavan", finish_cases()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, d in files().items():
        path = OUT / name
        text = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
        assert len(d["cases"]) >= 20, (name, len(d["cases"]))
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{d['command']}: {len(d['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
