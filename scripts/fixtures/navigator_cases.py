#!/usr/bin/env python3
"""Proje gezgini (docs/adr/0778), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.navigator.read.json; `--check` compares and writes nothing. On top of the sheet set's
frames and their recipes (`view_recipe_sheet_cases.py`, ADR 0347 and 0774, whose book, views and frame rules this
reuses):

- a level: its id, name and elevation, the objects on it (an opening on its wall's, a tag on its object's level), the
  named views whose recipe names it (in the views' order) and the maps that draw it (a frame's `level`: its view's
  recipe's, else its own; in the book's order); `unleveled` counts the objects on no level or on a level the project
  does not have;
- a view: its stable id and name, its recipe's level, version and scale (none without a recipe), the maps linked to it
  (a frame's `view`, kept when the view is gone);
- a sheet: its id, name, series and maps (item, name, view, missing, level, scale: the set's);
- a schedule: a table reading the model (here a column schedule `cad.schedules.create` writes, every column of the
  drawing a row), what it lists and how many objects;
- given `uid`: its kind; its level (none when on no level or on one the project does not have); the views drawing it
  (the box of the view's area, its corners turned by its twist, meets the object's box, and its recipe, or none, keeps
  it: level, phase, category); the maps drawing it (the set's `shows`); the schedules listing it; the tags naming it.
  The objects' boxes are kept off the frames' and views' edges by at least half a metre (asserted), so a box needs no
  exact metric; a tag's box is its anchor's neighbourhood.
- refusals: an id not in the drawing, at `uid`.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sheet_set_cases as ssc  # noqa: E402
import view_recipe_sheet_cases as vrs  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.navigator.read.json"
P, uid_of = ssc.P, ssc.uid_of
ANY = ssc.ANY
LEVELS = vrs.LEVELS
KNOWN = {l["id"] for l in LEVELS}
TABLE = 11  # the schedule `cad.schedules.create` writes: the next slot after the setup's objects

# id: (kind, its box, its own level, its category, its phase); a tag's level and phase are its object's.
OBJECTS = {i: (k, b, lv, cat, vrs.PHASES.get(i)) for i, (k, _ly, _f, b, lv, cat) in vrs.SEED.items()}
OBJECTS[9] = ("tag", [1.9, 2.4, 2.1, 2.6], "zemin", "tags", None)
OBJECTS[10] = ("column", [508.8, 499.8, 509.2, 500.2], "kat-1", None, None)
OBJECTS[TABLE] = ("table", [0.0, -64.0, 6.0, -60.0], None, None, None)
TAGS = {9: 1}  # tag → the object it names
IDS = sorted(OBJECTS)
COLUMNS = [i for i in IDS if OBJECTS[i][0] == "column"]

EXTRA_VIEWS = [
    vrs.named("Uzak", P(500, 500), 30.0, 30.0, None, "v-uzak"),
    {"id": "v-donuk", "name": "Dönük", "center": P(500, 500), "width": 20.0, "height": 2.0, "twist": math.pi / 2},
]
VIEWS = vrs.VIEWS + EXTRA_VIEWS

SETUP = copy.deepcopy(vrs.SETUP)
SETUP["name"] = "Proje gezgini"
SETUP["settings"]["namedViews"] = VIEWS
FIXED = {i: f"0192f5a1-7030-7000-8000-0000000780{i:02x}" for i in range(1, 11)}  # the setup's persistent ids
SETUP["entities"] += [
    {"kind": "tag", "id": 9, "layerId": "mimari", "attrs": {}, "of": FIXED[1], "at": P(2, 2.5), "text": "K01"},
    {"kind": "column", "id": 10, "layerId": "mimari", "attrs": {},
     "at": P(509, 500), "width": 0.4, "depth": 0.4, "arch": {"level": "kat-1"}},
]
SETUP["version"] = 2  # a v2 setup keeps its persistent ids
SETUP["uids"] = [FIXED[e["id"]] for e in SETUP["entities"]]
PRELUDE = copy.deepcopy(vrs.PRELUDE) + [
    {"op": "execute", "command": "cad.schedules.create", "note": "kolon çizelgesi: kurulum çizelge yazamaz",
     "input": {"layerId": "mimari", "at": P(0, -60), "of": "columns"},
     "result": ANY, "expect": {"revision": "changed"}},
]


def view_box(v):
    s, c = math.sin(v["twist"]), math.cos(v["twist"])
    hw, hh = v["width"] / 2, v["height"] / 2
    pts = [(v["center"]["x"] + x * c - y * s, v["center"]["y"] + x * s + y * c)
           for x, y in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]]
    return [min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)]


def clear_of(a, b, margin=0.5):
    """Either inside with `margin` to spare or apart by `margin`: no tie for a box metric to decide."""
    inside = a[0] >= b[0] + margin and a[1] >= b[1] + margin and a[2] <= b[2] - margin and a[3] <= b[3] - margin
    apart = a[2] < b[0] - margin or b[2] < a[0] - margin or a[3] < b[1] - margin or b[3] < a[1] - margin
    return inside or apart


def keeps(level, phase, hidden, i):
    """A view's recipe (level, phase view, hidden categories) keeps object `i`."""
    _k, _b, own, cat, ph = OBJECTS[i]
    on = level is None or own is None or own == level or own not in KNOWN
    return on and cat not in hidden and ph not in vrs.HIDDEN[phase or "all"]


class Nav:
    def __init__(self, views=None):
        self.views = copy.deepcopy(VIEWS if views is None else views)
        self.book = vrs.Book(self.views)

    def frames(self):
        """(sheet, item, frame, level, hidden, phase), in the book's order."""
        out = self.book.output()
        for s, sheet in zip(self.book.sheets, out["sheets"]):
            for item, f in zip(s["items"], sheet["frames"]):
                _shown, view, _missing = self.book.camera(item)
                r = (view or {}).get("recipe") or {}
                yield s, item, f, f.get("level"), r.get("hidden", []), r.get("phase")

    def output(self, uid=None):
        out = self.book.output()
        levels = []
        for l in LEVELS:
            levels.append({
                "id": l["id"], "name": l["name"], "elevation": l["elevation"],
                "objects": sum(1 for i in IDS if OBJECTS[i][2] == l["id"]),
                "views": [v["id"] for v in self.views if ((v.get("recipe") or {}).get("level") or {}).get("id") == l["id"]],
                "maps": [{"sheet": s["id"], "item": item["id"]} for s, item, f, lv, *_ in self.frames() if lv == l["id"]],
            })
        unleveled = sum(1 for i in IDS if OBJECTS[i][2] not in KNOWN)
        views = []
        for v in self.views:
            r = v.get("recipe")
            x = {"id": v["id"], "name": v["name"]}
            if r and r.get("level"):
                x["level"] = r["level"]["id"]
            if r:
                x["recipe"] = r["version"]
                if r.get("scale"):
                    x["scale"] = r["scale"]
            x["maps"] = [{"sheet": s["id"], "item": item["id"]} for s, item, f, *_ in self.frames() if f.get("view") == v["id"]]
            views.append(x)
        sheets = []
        for s in out["sheets"]:
            maps = []
            for f in s["frames"]:
                m = {"item": f["item"], "name": f["name"]}
                for k in ("view", "missing", "level", "scale"):
                    if k in f:
                        m[k] = float(f[k]) if k == "scale" else f[k]
                maps.append(m)
            sheets.append({"id": s["id"], "name": s["name"], "series": s["series"], "maps": maps})
        schedules = [{"uid": f"$uidOf:{TABLE}", "of": "columns", "objects": len(COLUMNS)}]
        res = {"levels": levels, "unleveled": unleveled, "views": views, "sheets": sheets, "schedules": schedules}
        if uid is not None:
            kind, box, own, _cat, _ph = OBJECTS[uid]
            for v in self.views:
                assert clear_of(box, view_box(v)), (uid, v["name"])
            for _s, _item, f, *_ in self.frames():
                if "bounds" in f:
                    assert clear_of(box, f["bounds"]), (uid, f["item"])
            obj = {"uid": f"$uidOf:{TABLE}" if uid == TABLE else uid_of(uid), "kind": kind}
            if own in KNOWN:
                obj["level"] = own
            obj["views"] = [v["id"] for v in self.views
                            if ssc.meets(box, view_box(v))
                            and keeps(((v.get("recipe") or {}).get("level") or {}).get("id"),
                                      (v.get("recipe") or {}).get("phase"), (v.get("recipe") or {}).get("hidden", []), uid)]
            obj["maps"] = [{"sheet": s["id"], "item": item["id"]} for s, item, f, lv, hidden, phase in self.frames()
                           if "bounds" in f and ssc.meets(box, f["bounds"]) and keeps(lv, phase, hidden, uid)]
            obj["schedules"] = [f"$uidOf:{TABLE}"] if uid in COLUMNS else []
            obj["tags"] = [uid_of(t) for t, of in TAGS.items() if of == uid]
            res["object"] = obj
        res["revision"] = "$current"
        return res


def step(nav, uid=None):
    inp = {} if uid is None else {"uid": f"$uidOf:{TABLE}" if uid == TABLE else uid_of(uid)}
    return {"op": "execute", "command": "cad.navigator.read", "input": inp,
            "result": {"status": "completed", "output": nav.output(uid), "warnings": []},
            "expect": {"revision": "same"}}


def refused(uid):
    return {"op": "execute", "command": "cad.navigator.read", "input": {"uid": uid},
            "result": ssc.refusal("entity_not_found",
                                  f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                                  "uid"),
            "expect": {"revision": "same"}}


def cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": copy.deepcopy(PRELUDE) + steps})

    add("Ağaç: katlar, görünümler, paftalar ve çizelge bağlarıyla", [step(Nav())])
    add("Zemin kolonu: zemin görünümleri ve haritaları, çizelgesi ve etiketi", [step(Nav(), 1)])
    add("1. kat kolonu: 1. kat görünümü ve haritaları, katsız tarifli görünüm", [step(Nav(), 2)])
    add("Katsız çizgi: her görünümde ve kutusundaki her haritada", [step(Nav(), 3)])
    add("Tarama: taramayı gizleyen zemin görünümünde ve haritasında yok", [step(Nav(), 4)])
    add("Uzaktaki kolon: yalnız alanı onu kapsayan görünümlerde, hiçbir haritada", [step(Nav(), 5)])
    add("Projede olmayan katın kolonu: katı yok, her görünümde", [step(Nav(), 6)])
    add("Yıkılacak kolon: yeni durum görünümünde ve haritasında yok", [step(Nav(), 7)])
    add("Yeni kolon: yıkım planında yok", [step(Nav(), 8)])
    add("Etiket: nesnesinin katında, etiketler kategorisi", [step(Nav(), 9)])
    add("Dönük görünüm: kutusu dar, yanındaki kolonu çizmez", [step(Nav(), 10)])
    add("Çizelge tablosu: katsız, alanı hiçbir görünümde değil", [step(Nav(), TABLE)])
    gone = [v for v in VIEWS if v["name"] != "Zemin"]
    add("Silinen görünüm: bağlı harita kayıp, zemini bilinmez, katı yok",
        [vrs.views_edit({"kind": "delete", "name": "Zemin"}), step(Nav(gone))])
    add("Silinen görünüm: zemin kolonu kayıp haritada sayılmaz",
        [vrs.views_edit({"kind": "delete", "name": "Zemin"}), step(Nav(gone), 1)])
    up = copy.deepcopy(VIEWS)
    up[0]["recipe"] = {"version": 2, "level": {"id": "kat-1"}, "hidden": ["hatches"], "scale": 100}
    add("Tarifin katı değişince görünüm ve haritası 1. kat altında",
        [vrs.views_edit({"kind": "recipe", "name": "Zemin",
                         "recipe": {"level": {"id": "kat-1"}, "hidden": ["hatches"], "scale": 100}}), step(Nav(up))])
    add("Tarifin katı değişince zemin kolonu o görünümde değil, 1. kat kolonu var",
        [vrs.views_edit({"kind": "recipe", "name": "Zemin",
                         "recipe": {"level": {"id": "kat-1"}, "hidden": ["hatches"], "scale": 100}}), step(Nav(up), 2)])
    up2 = copy.deepcopy(VIEWS)
    up2[0]["recipe"] = {"version": 2, "level": {"id": "zemin"}, "scale": 100}
    add("Gizlenen tür kalkınca tarama zemin görünümünde ve haritasında",
        [vrs.views_edit({"kind": "recipe", "name": "Zemin", "recipe": {"level": {"id": "zemin"}, "scale": 100}}),
         step(Nav(up2), 4)])
    up3 = copy.deepcopy(VIEWS)
    up3[5]["recipe"] = {"version": 1, "level": {"id": "zemin"}, "scale": 200}
    add("Bağsız görünüme tarif: Uzak yalnız zemini çizer, zemin katının altında",
        [vrs.views_edit({"kind": "recipe", "name": "Uzak", "recipe": {"level": {"id": "zemin"}, "scale": 200}}),
         step(Nav(up3)), step(Nav(up3), 10), step(Nav(up3), 5)])
    add("Etiketin nesnesi: etiket kolonun satırında", [step(Nav(), 1), step(Nav(), 9)])
    add("Okuma çizimi değiştirmez: iki kez aynı cevap", [step(Nav()), step(Nav())])
    add("Çizimde olmayan kimlik reddedilir", [refused("01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4eff")])
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.navigator.read", "commandVersion": 1,
           "title": "Proje gezgini (ADR 0778)",
           "note": "scripts/fixtures/navigator_cases.py yazar; ADR 0347, 0774 ve 0778'in kurallarından, KentOS kodu olmadan.",
           "setup": SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"cad.navigator.read: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
