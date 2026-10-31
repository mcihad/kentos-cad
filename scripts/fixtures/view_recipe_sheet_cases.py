#!/usr/bin/env python3
"""Pafta seti with view recipes (docs/adr/0774 §3, §4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.sheets.set.recipe.json; `--check` compares and writes nothing. On top of ADR 0347's
rules (`sheet_set_cases.py`, whose frame and corner rules this reuses):

- a map linked to a named view (`source: viewLink`) draws with the view's camera: its centre, and its recipe's scale
  when the recipe gives one; the frame says the view's stable id (`view`), its recipe's version (`recipe`), the level it
  draws (the recipe's, else the map's own) and, when the view is gone, `missing` (no ground: `unknown`);
- given objects, a frame `shows` an object only when it draws it: its box meets the frame's ground, it is on the frame's
  level or on none (or its level is not one of the project's), and its category is not one the recipe leaves out
  (a hatch: `hatches`; a column has no category, a line neither).
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sheet_set_cases as ssc  # noqa: E402  (frames, sheets, corners, series)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sheets.set.recipe.json"
P, uid_of, layer = ssc.P, ssc.uid_of, ssc.layer
ANY = ssc.ANY
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]

# kind, layer, fields, its box, its level, its category.
SEED = {
    1: ("column", "mimari", {"at": P(2, 2), "width": 0.4, "depth": 0.4, "arch": {"level": "zemin"}}, [1.8, 1.8, 2.2, 2.2], "zemin", None),
    2: ("column", "mimari", {"at": P(3, 2), "width": 0.4, "depth": 0.4, "arch": {"level": "kat-1"}}, [2.8, 1.8, 3.2, 2.2], "kat-1", None),
    3: ("line", "cizim", {"a": P(1, 1), "b": P(4, 1)}, [1, 1, 4, 1], None, None),
    4: ("hatch", "cizim", {"ring": [P(5, 0), P(7, 0), P(7, 2), P(5, 2)], "pattern": {"type": "solid", "angle": 0, "spacing": 1}},
        [5, 0, 7, 2], None, "hatches"),
    5: ("column", "mimari", {"at": P(500, 500), "width": 0.4, "depth": 0.4, "arch": {"level": "zemin"}}, [499.8, 499.8, 500.2, 500.2], "zemin", None),
    6: ("column", "mimari", {"at": P(4, 2), "width": 0.4, "depth": 0.4, "arch": {"level": "çatı"}}, [3.8, 1.8, 4.2, 2.2], "çatı", None),
    7: ("column", "mimari", {"at": P(2, 0), "width": 0.4, "depth": 0.4, "arch": {"level": "zemin", "phase": "demolish"}},
        [1.8, -0.2, 2.2, 0.2], "zemin", None),
    8: ("column", "mimari", {"at": P(3, 0), "width": 0.4, "depth": 0.4, "arch": {"level": "zemin", "phase": "new"}},
        [2.8, -0.2, 3.2, 0.2], "zemin", None),
}
PHASES = {7: "demolish", 8: "new"}
# How a phase view draws a phase (docs/adr/0346 §2): only `hidden` keeps an object off the map.
HIDDEN = {"existing": {"new", "temporary"}, "demolition": {"new", "temporary"}, "proposed": {"demolish"}, "all": set()}
IDS = sorted(SEED)


def named(name, center, w, h, recipe=None, vid=None):
    v = {**({"id": vid} if vid else {}), "name": name, "center": center, "width": w, "height": h, "twist": 0.0}
    if recipe is not None:
        v["recipe"] = recipe
    return v


VIEWS = [
    named("Zemin", P(3, 1), 10.0, 6.0, {"version": 1, "level": {"id": "zemin"}, "hidden": ["hatches"], "scale": 100}, "v-zemin"),
    named("Kat 1", P(3, 1), 10.0, 6.0, {"version": 2, "level": {"id": "kat-1"}, "scale": 50}, "v-kat1"),
    named("Gider", P(3, 1), 10.0, 6.0, {"version": 1, "scale": 50}, "v-gider"),
    named("Yeni", P(3, 1), 10.0, 6.0, {"version": 1, "level": {"id": "zemin"}, "phase": "proposed", "scale": 100}, "v-yeni"),
    named("Yıkım", P(3, 1), 10.0, 6.0, {"version": 1, "level": {"id": "zemin"}, "phase": "demolition", "scale": 100}, "v-yikim"),
]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Görünüş tarifli pafta seti",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "arch": {"levels": LEVELS}, "namedViews": VIEWS},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("mimari", "Mimari"), layer("cizim", "Çizim")],
    "activeLayer": "mimari",
    "styles": {"items": [], "categories": []},
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": {}, **f} for i, (k, ly, f, *_rest) in sorted(SEED.items())],
}


def linked(item, view):
    out = copy.deepcopy(item)
    out["kind"]["source"] = {"type": "viewLink", "view": view}
    return out


def with_level(item, level):
    out = copy.deepcopy(item)
    out["kind"]["level"] = {"id": level}
    return out


BOOK = [
    ssc.sheet("a-101", "A-101 Zemin uygulama", [linked(ssc.frame("zemin", "Zemin", center=P(0, 0), scale=500), VIEWS[0])]),
    ssc.sheet("a-102", "A-102 1. kat", [linked(ssc.frame("kat1", "Kat 1", center=P(0, 0), scale=500), VIEWS[1])]),
    ssc.sheet("a-103", "A-103 Kendi katı", [with_level(ssc.frame("kendi", "Kendi", center=P(3, 1), scale=50), "kat-1")]),
    ssc.sheet("a-104", "A-104 Bağsız", [ssc.frame("bagsiz", "Bağsız", center=P(3, 1), scale=50)]),
    ssc.sheet("a-105", "A-105 Katsız tarif", [with_level(linked(ssc.frame("gider", "Gider", center=P(0, 0), scale=500), VIEWS[2]), "kat-1")]),
    ssc.sheet("a-106", "A-106 Yeni durum", [linked(ssc.frame("yeni", "Yeni", center=P(0, 0), scale=500), VIEWS[3])]),
    ssc.sheet("a-107", "A-107 Yıkım", [linked(ssc.frame("yikim", "Yıkım", center=P(0, 0), scale=500), VIEWS[4])]),
]
PRELUDE = [{"op": "execute", "command": "cad.sheets.edit", "note": "pafta kitabı: kurulum kitap veremez",
            "input": {"ops": [{"op": "addSheet", "sheet": s} for s in BOOK]},
            "result": ANY, "expect": {"revision": "changed"}}]


class Book:
    def __init__(self, views=None):
        self.sheets = copy.deepcopy(BOOK)
        self.views = copy.deepcopy(VIEWS if views is None else views)

    def camera(self, item):
        """The item as it draws: a linked one with its view's centre and its recipe's scale; none for a lost link."""
        src = item["kind"].get("source")
        if not src:
            return item, None, False
        key = src["view"]["id"]
        view = next((v for v in self.views if v.get("id") == key), None)
        if view is None:
            return None, None, True
        out = copy.deepcopy(item)
        out["kind"]["view"]["center"] = view["center"]
        out["kind"]["view"]["scale"] = view["recipe"]["scale"]
        return out, view, False

    def output(self, uids=None):
        sheets = []
        drawn = []
        for s in self.sheets:
            frames = []
            for item in s["items"]:
                shown, view, missing = self.camera(item)
                f = {"item": item["id"], "name": item["name"]}
                if shown is not None:
                    f["scale"] = shown["kind"]["view"]["scale"]
                    c = ssc.corners(shown)
                    f["corners"] = c
                    xs, ys = [p["x"] for p in c], [p["y"] for p in c]
                    f["bounds"] = [min(xs), min(ys), max(xs), max(ys)]
                f["marks"] = []
                if "source" in item["kind"]:
                    f["view"] = item["kind"]["source"]["view"]["id"]
                if missing:
                    f["missing"] = True
                recipe = view.get("recipe") if view else None
                level = (recipe or {}).get("level", {}).get("id") or item["kind"].get("level", {}).get("id")
                if level:
                    f["level"] = level
                if recipe:
                    f["recipe"] = recipe["version"]
                frames.append(f)
                drawn.append((s, item, f, level, (recipe or {}).get("hidden", []), (recipe or {}).get("phase")))
            sheets.append({"id": s["id"], "name": s["name"], "series": ssc.series_of(s["name"]), "frames": frames})
        series = []
        for s in sheets:
            got = next((x for x in series if x["series"] == s["series"]), None)
            if got:
                got["sheets"].append(s["id"])
            else:
                x = {"series": s["series"]}
                if ssc.TITLES.get(s["series"]):
                    x["title"] = ssc.TITLES[s["series"]]
                x["sheets"] = [s["id"]]
                series.append(x)
        series.sort(key=lambda x: (x["series"] == ssc.OTHER, x["series"]))
        out = {"series": series, "sheets": sheets}
        if uids is not None:
            known = {l["id"] for l in LEVELS}
            affected = []
            for s, item, fr, level, hidden, phase in drawn:
                reasons, objs = [], []
                if "bounds" in fr:
                    def draws(i):
                        _, _, _, b, own, cat = SEED[i]
                        on = level is None or own is None or own == level or own not in known
                        kept = PHASES.get(i) not in HIDDEN[phase or "all"]
                        return ssc.meets(b, fr["bounds"]) and on and cat not in hidden and kept
                    hit = [i for i in uids if draws(i)]
                    if hit:
                        reasons.append("shows")
                        objs += hit
                if "bounds" not in fr and uids:
                    reasons.append("unknown")
                if reasons:
                    affected.append({"sheet": s["id"], "item": item["id"], "reasons": reasons,
                                     "uids": [uid_of(i) for i in objs]})
            out["affected"] = affected
        out["revision"] = "$current"
        return out


def step(book, uids=None):
    inp = {} if uids is None else {"uids": [uid_of(u) for u in uids]}
    return {"op": "execute", "command": "cad.sheets.set", "input": inp,
            "result": {"status": "completed", "output": book.output(uids), "warnings": []},
            "expect": {"ids": IDS, "revision": "same"}}


def views_edit(edit):
    return {"op": "execute", "command": "cad.views.edit", "input": {"edit": edit}, "result": ANY,
            "expect": {"revision": "changed"}}


def cases():
    out = []

    def add(name, steps):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "steps": copy.deepcopy(PRELUDE) + steps})

    add("Bağlı haritalar görünümün kimliğini, tarif sürümünü ve katını söyler", [step(Book())])
    add("Tarifin ölçeği haritanın ölçeğidir; görünümün ortası haritanın ortası", [step(Book(), [3])])
    add("Zemin kolonu: zemin ve bağsız haritalar gösterir, 1. kat haritaları göstermez", [step(Book(), [1])])
    add("1. kat kolonu: yalnız 1. kat haritaları ve bağsız harita", [step(Book(), [2])])
    add("Katsız çizgi her katın haritasında", [step(Book(), [3])])
    add("Gizlenen tür: tarama zemin haritasında yok, ötekilerde var", [step(Book(), [4])])
    add("Projede olmayan katın nesnesi her haritada", [step(Book(), [6])])
    add("Çerçevenin dışındaki nesne hiçbir haritada", [step(Book(), [5])])
    add("Birkaç nesne birlikte: her harita yalnız çizdiklerini sayar", [step(Book(), [1, 2, 3, 4])])
    add("Girdinin sırası korunur", [step(Book(), [4, 3, 2, 1])])
    lost = [v for v in VIEWS if v["name"] != "Zemin"]
    add("Silinen görünüme bağlı harita: kayıp, zemini bilinmez", [views_edit({"kind": "delete", "name": "Zemin"}), step(Book(lost))])
    add("Silinen görünüme bağlı harita nesneleri bilinmeyen olarak etkiler", [views_edit({"kind": "delete", "name": "Zemin"}),
                                                                             step(Book(lost), [1, 3])])
    up = copy.deepcopy(VIEWS)
    up[0]["recipe"] = {"version": 2, "level": {"id": "zemin"}, "scale": 100}
    add("Tarif değişince harita yeni tarifle: tarama artık gösterilir, sürüm 2",
        [views_edit({"kind": "recipe", "name": "Zemin", "recipe": {"level": {"id": "zemin"}, "scale": 100}}), step(Book(up), [4])])
    up2 = copy.deepcopy(VIEWS)
    up2[0]["recipe"] = {"version": 2, "level": {"id": "kat-1"}, "hidden": ["hatches"], "scale": 100}
    add("Tarifin katı değişince zemin haritası 1. kat kolonunu gösterir",
        [views_edit({"kind": "recipe", "name": "Zemin",
                     "recipe": {"level": {"id": "kat-1"}, "hidden": ["hatches"], "scale": 100}}), step(Book(up2), [1, 2])])
    up3 = copy.deepcopy(VIEWS)
    up3[1]["recipe"] = {"version": 3, "level": {"id": "kat-1"}, "scale": 20}
    add("Tarifin ölçeği 1:20: zemin küçülür, uzaktaki kolon dışarıda kalır",
        [views_edit({"kind": "recipe", "name": "Kat 1", "recipe": {"level": {"id": "kat-1"}, "scale": 20}}), step(Book(up3), [2, 3])])
    add("Katsız tarif: harita kendi katını (1. kat) çizer", [step(Book(), [1, 2])])
    add("Yeni durum tarifi yıkılacak kolonu çizmez, yeniyi çizer", [step(Book(), [7, 8])])
    add("Yıkım planı tarifi yeni kolonu çizmez, yıkılacağı çizer", [step(Book(), [7, 8])])
    add("Evresiz nesne her evre tarifinde", [step(Book(), [1])])
    add("Bütün nesneler birlikte", [step(Book(), IDS)])
    up5 = copy.deepcopy(VIEWS)
    up5[3]["recipe"] = {"version": 2, "level": {"id": "zemin"}, "phase": "existing", "scale": 100}
    add("Evre tarifi Mevcut duruma dönünce yeni kolon gizlenir, yıkılacak görünür",
        [views_edit({"kind": "recipe", "name": "Yeni",
                     "recipe": {"level": {"id": "zemin"}, "phase": "existing", "scale": 100}}), step(Book(up5), [7, 8])])
    add("Boş nesne listesi hiçbir haritayı etkilemez", [step(Book(), [])])
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.sheets.set", "commandVersion": 1,
           "title": "Pafta seti ve görünüş tarifi (ADR 0774)",
           "note": "scripts/fixtures/view_recipe_sheet_cases.py yazar; ADR 0347 ve 0774'ün kurallarından, KentOS kodu olmadan.",
           "setup": SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"cad.sheets.set (tarif): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
