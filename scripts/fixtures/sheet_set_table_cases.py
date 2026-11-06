#!/usr/bin/env python3
"""`cad.sheets.set`'s `table` reason (docs/adr/1090 §6), shared cases from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.sheets.set.table.json; `--check` compares and writes nothing. The rules, as ADR 1090 §6
and ADR 0347 §1 give them:

- a frame's ground (a fixed view with a centre, no turn here): the frame's content (no padding, no label band) is
  `width × scale / 1e6` metres east–west and `height × scale / 1e6` north–south around the centre; its box closed;
- given `uids` (each found, a repeated one counted once; the first missing one `entity_not_found` at `uids[i]`), a
  model table **reads** one of them when it is a coordinate, area or attribute table whose `objects` name it, a block
  schedule (`blocks`) counting it (an insert; its block's name among `names`, or any insert when `names` is empty), a
  schedule (`schedule`) one of whose rows' `members` names it, or one of its cell `fields` reads that object; a table
  that is itself among the uids reads nothing here (it is an object of the change, `shows` judges it);
- a frame is affected by `table` when it draws such a table: the table's box (its outline: its top left `p`, its
  columns' widths east, its rows' heights south) meets the ground's box and the frame prints the table's layer (the
  drawing's visibility; a frame's layer override `frozen` leaves it out of that frame only); the frame's `uids` gain
  the objects the shown tables read, its `tables` the shown tables in the drawing's order;
- reasons in the order shows, table, marked, unknown; a frame's objects in the input's order, each once; a frame of no
  known ground (atlas) is `unknown` with any object and never `table`.
"""

import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
COMMAND = "cad.sheets.set"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
TITLES = {"A-1": "Planlar", "A-5": "Detaylar"}
OTHER = "Diğer"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def U(i):
    return f"0192f5c0-7a01-7000-8000-{i:012d}"


def layer(id, name, visible=True):
    return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


KAPI = "0192f5c0-7a01-7000-8000-00000000b001"
PENCERE = "0192f5c0-7a01-7000-8000-00000000b002"
BLOCKS = [
    {"id": KAPI, "name": "Kapi", "base": P(0, 0), "units": "m",
     "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0.9, 0)}]},
    {"id": PENCERE, "name": "Pencere", "base": P(0, 0), "units": "m",
     "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(1.2, 0)}]},
]
NAMES = {KAPI: "Kapi", PENCERE: "Pencere"}


def table(p, rows, columns, source=None, fields=None):
    cells = [["" for _ in columns] for _ in rows]
    cells[0] = [f"S{k + 1}" for k in range(len(columns))]
    t = {"p": p, "rotation": 0.0, "height": 0.2, "rows": rows, "columns": columns, "cells": cells, "header": True}
    if source:
        t["source"] = source
    if fields:
        t["fields"] = fields
    return t


def field(row, col, uid, value="length"):
    return {"row": row, "col": col, "source": {"kind": "object", "uid": uid, "value": value}}


# id: (kind, layer, own fields)
SEED = {
    1: ("line", "cizim", {"a": P(0, 0), "b": P(4, 0)}),
    2: ("polygon", "cizim", {"pts": [P(500, 500), P(504, 500), P(504, 503), P(500, 503)]}),
    3: ("circle", "cizim", {"c": P(520, 500), "r": 1.0}),
    4: ("insert", "cizim", {"block": KAPI, "p": P(540, 500), "scale": 1.0, "rotation": 0.0}),
    5: ("insert", "cizim", {"block": PENCERE, "p": P(560, 500), "scale": 1.0, "rotation": 0.0}),
    6: ("line", "cizim", {"a": P(600, 500), "b": P(601, 500)}),
    # T1: the polygon's coordinates, a field reading the line; in the plan.
    7: ("table", "tablo", table(P(-8, 5), [0.5, 0.5, 0.5], [2.0, 2.0], {"kind": "coordinates", "objects": [U(2)]},
                                [field(1, 1, U(1))])),
    # T2: the circle's and the polygon's areas; in both detail frames' ground, its layer frozen in the second.
    8: ("table", "tablo", table(P(99, 1), [0.5, 0.5], [1.0, 1.0], {"kind": "areas", "objects": [U(3), U(2)]})),
    # T3: every insert counted; in the plan.
    9: ("table", "tablo", table(P(2, -2), [0.5, 0.5, 0.5], [2.0, 1.0], {"kind": "blocks"})),
    # T4: the doors only; in the section.
    10: ("table", "tablo", table(P(196, 2), [0.5, 0.5], [2.0, 1.0], {"kind": "blocks", "names": ["Kapi"]})),
    # T5: a schedule whose rows list the circle and the window; in the section.
    11: ("table", "tablo", table(P(200, -1), [0.5, 0.5, 0.5], [2.0, 1.0],
                                 {"kind": "schedule", "of": "fixtures", "columns": [{"field": "type"}, {"field": "count"}],
                                  "members": [[U(3)], [U(5)]]})),
    # T6: no source, a cell reading the line; on the drawing layer, in both detail frames.
    12: ("table", "cizim", table(P(100.5, -0.2), [0.5, 0.5], [1.0, 0.5], None, [field(1, 0, U(1))])),
    # T7: on a hidden layer; in the plan's ground, never drawn.
    13: ("table", "gizli", table(P(-8, -3), [0.5], [1.0], {"kind": "coordinates", "objects": [U(3)]})),
    # T8: touches the plan's east edge.
    14: ("table", "tablo", table(P(9, 0), [0.5], [1.0], {"kind": "coordinates", "objects": [U(2)]})),
    # T9: a centimetre past the plan's east edge.
    15: ("table", "tablo", table(P(9.01, 2), [0.5], [1.0], {"kind": "coordinates", "objects": [U(2)]})),
}
IDS = sorted(SEED)
LAYERS = [layer("cizim", "Çizim"), layer("tablo", "Tablo"), layer("gizli", "Gizli", visible=False)]
VISIBLE = {ly["id"]: ly["visible"] for ly in LAYERS}
SETUP = {
    "format": "kentos.document", "version": 2, "name": "Tablolu pafta seti",
    "uids": [U(i) for i in IDS],
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 100.0, "workspace": "cad"},
    "origin": P(0, 0),
    "layers": LAYERS,
    "activeLayer": "cizim",
    "styles": {"items": [], "categories": []},
    "blocks": BLOCKS,
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": {}, **f} for i, (k, ly, f) in sorted(SEED.items())],
}


def box(i):
    k, _, f = SEED[i]
    if k == "line":
        pts = [f["a"], f["b"]]
    elif k == "polygon":
        pts = f["pts"]
    elif k == "circle":
        c, r = f["c"], f["r"]
        return [c["x"] - r, c["y"] - r, c["x"] + r, c["y"] + r]
    elif k == "insert":
        block = next(b for b in BLOCKS if b["id"] == f["block"])
        pts = [P(f["p"]["x"] + q["x"], f["p"]["y"] + q["y"]) for e in block["entities"] for q in (e["a"], e["b"])]
    elif k == "table":
        p, w, h = f["p"], sum(f["columns"]), sum(f["rows"])
        return [p["x"], p["y"] - h, p["x"] + w, p["y"]]
    xs, ys = [p["x"] for p in pts], [p["y"] for p in pts]
    return [min(xs), min(ys), max(xs), max(ys)]


def meets(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


def reads(t, uid):
    """Whether table `t` reads the object `uid` (ADR 1090 §6)."""
    _, _, f = SEED[t]
    i = next(k for k in IDS if U(k) == uid)
    kind, _, own = SEED[i]
    src = f.get("source") or {}
    by_source = False
    if src.get("kind") in ("coordinates", "areas", "attributes"):
        by_source = uid in src["objects"]
    elif src.get("kind") == "blocks":
        by_source = kind == "insert" and (not src.get("names") or NAMES[own["block"]] in src["names"])
    elif src.get("kind") == "schedule":
        by_source = any(uid in row for row in src.get("members", []))
    by_field = any(x["source"]["uid"] == uid for x in f.get("fields", []))
    return by_source or by_field


# ── The book ───────────────────────────────────────────────────────────────────────────────────────────────────

W, H = 180000, 120000


def frame(item, name, *, center=None, scale=50, left=20000, frozen=()):
    view = ({"policy": {"scale": 500, "type": "fixed"}, "rotation": 0, "type": "atlas"} if center is None else
            {"center": center, "rotation": 0, "scale": scale, "type": "fixed"})
    kind = {"clipToAtlasFeature": False, "grids": [], "labelBand": 0, "layers": {"type": "all"},
            "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
            "type": "map", "view": view}
    if frozen:
        kind["layerOverrides"] = [{"layer": ly, "frozen": True} for ly in frozen]
    return {"bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": H, "left": left, "top": 10000, "width": W}, "hidden": False, "id": item,
            "kind": kind, "locked": False, "name": name, "opacity": 100, "padding": 0, "printable": True,
            "rotation": 0}


def sheet(id, name, items):
    return {"export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "png"}, "guides": [], "id": id,
            "items": items, "name": name,
            "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                     "paper": "a3", "size": {"height": 297000, "width": 420000}},
            "snapGrid": {"enabled": False, "spacing": 5000, "visible": False}, "variables": [], "variants": []}


BOOK = [
    sheet("a-101", "A-101 Plan", [frame("plan", "Plan", center=P(0, 0), scale=100)]),
    sheet("a-501", "A-501 Detaylar", [frame("detay", "Detay", center=P(100, 0), scale=20),
                                      frame("detay-donuk", "Detay tablosuz", center=P(100, 0), scale=20, left=210000,
                                            frozen=("tablo",))]),
    sheet("s-101", "S-101 Kesit", [frame("kesit", "Kesit", center=P(200, 0), scale=50)]),
    sheet("kapak", "Kapak", [frame("anahtar", "Anahtar")]),
]
PRELUDE = [{"op": "execute", "command": "cad.sheets.edit", "note": "pafta kitabı: kurulum kitap vermez",
            "input": {"ops": [{"op": "addSheet", "sheet": s} for s in BOOK]},
            "result": ANY, "expect": {"revision": "changed"}}]


def series_of(name):
    t = name.strip()
    k = 0
    while k < len(t) and t[k].isascii() and t[k].isalpha():
        k += 1
    if k == 0:
        return OTHER
    rest = t[k:]
    if rest[:1] in ("-", ".", " "):
        rest = rest[1:]
    if rest[:1].isascii() and rest[:1].isdigit():
        return f"{t[:k].upper()}-{rest[0]}"
    return OTHER


def ground(item):
    view = item["kind"]["view"]
    if view["type"] != "fixed":
        return None
    c, s = view["center"], view["scale"]
    hw, hh = W / 2 * s / 1e6, H / 2 * s / 1e6
    return [P(c["x"] - hw, c["y"] + hh), P(c["x"] + hw, c["y"] + hh), P(c["x"] + hw, c["y"] - hh),
            P(c["x"] - hw, c["y"] - hh)]


def prints(item, i):
    ly = SEED[i][1]
    frozen = {o["layer"] for o in item["kind"].get("layerOverrides", []) if o.get("frozen")}
    return VISIBLE[ly] and ly not in frozen


class Drawing:
    def __init__(self, book=True):
        self.sheets = copy.deepcopy(BOOK) if book else []
        self.ids = list(IDS)

    def output(self, uids=None):
        sheets = []
        for s in self.sheets:
            frames = []
            for item in s["items"]:
                f = {"item": item["id"], "name": item["name"]}
                view = item["kind"]["view"]
                if view["type"] == "fixed":
                    f["scale"] = view["scale"]
                c = ground(item)
                if c:
                    f["corners"] = c
                    xs, ys = [p["x"] for p in c], [p["y"] for p in c]
                    f["bounds"] = [min(xs), min(ys), max(xs), max(ys)]
                f["marks"] = []
                frames.append(f)
            sheets.append({"id": s["id"], "name": s["name"], "series": series_of(s["name"]), "frames": frames})
        series = []
        for s in sheets:
            got = next((x for x in series if x["series"] == s["series"]), None)
            if got:
                got["sheets"].append(s["id"])
            else:
                x = {"series": s["series"]}
                if TITLES.get(s["series"]):
                    x["title"] = TITLES[s["series"]]
                x["sheets"] = [s["id"]]
                series.append(x)
        series.sort(key=lambda x: (x["series"] == OTHER, x["series"]))
        out = {"series": series, "sheets": sheets}
        if uids is not None:
            named = []
            for u in uids:
                if u not in named:
                    named.append(u)
            ids = [next(i for i in self.ids if U(i) == u) for u in named]
            tables = [t for t in self.ids if SEED[t][0] == "table" and t not in ids
                      and any(reads(t, u) for u in named)]
            affected = []
            for s, fs in zip(self.sheets, sheets):
                for item, fr in zip(s["items"], fs["frames"]):
                    reasons, objs, shown = [], [], []
                    if "bounds" in fr:
                        b = fr["bounds"]
                        hit = [i for i in ids if meets(box(i), b) and prints(item, i)]
                        if hit:
                            reasons.append("shows")
                            objs += [U(i) for i in hit]
                        shown = [t for t in tables if meets(box(t), b) and prints(item, t)]
                        if shown:
                            reasons.append("table")
                            for t in shown:
                                objs += [u for u in named if reads(t, u) and u not in objs]
                    elif named:
                        reasons.append("unknown")
                    if reasons:
                        objs.sort(key=named.index)
                        a = {"sheet": s["id"], "item": item["id"], "reasons": reasons, "uids": objs}
                        if shown:
                            a["tables"] = [U(t) for t in shown]
                        affected.append(a)
            out["affected"] = affected
        out["revision"] = "$current"
        return out


def step(d, uids=None, op="execute"):
    inp = {} if uids is None else {"uids": [U(u) if isinstance(u, int) else u for u in uids]}
    out = d.output(None if uids is None else inp["uids"])
    return {"op": op, "command": COMMAND, "input": inp,
            "result": {"status": "completed", "output": None if op == "validate" else out, "warnings": []},
            "expect": {"ids": d.ids, "revision": "same"}}


def refused(d, uids, at):
    given = [U(x) if isinstance(x, int) else x for x in uids]
    u = given[at]
    return {"op": "execute", "command": COMMAND, "input": {"uids": given},
            "result": {"status": "failed", "error": {
                "code": "entity_not_found",
                "message": f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                "path": f"uids[{at}]"}},
            "expect": {"ids": d.ids, "revision": "same"}}


def delete(d, i):
    d.ids = [k for k in d.ids if k != i]
    return {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [U(i)]}, "result": ANY,
            "expect": {"revision": "changed"}}


def freeze_in(d, sheet_id, item_id, layer_id):
    """The frame's own override freezing a layer (docs/adr/0314 §1), by `cad.sheets.edit`."""
    s = next(s for s in d.sheets if s["id"] == sheet_id)
    item = next(x for x in s["items"] if x["id"] == item_id)
    item["kind"]["layerOverrides"] = [{"layer": layer_id, "frozen": True}]
    return {"op": "execute", "command": "cad.sheets.edit",
            "input": {"ops": [{"op": "setViewport", "id": item_id, "layerOverrides": item["kind"]["layerOverrides"]}]},
            "result": ANY, "expect": {"revision": "changed"}}


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps, book=True):
        assert len(name) > 10, name
        assert name not in [c["name"] for c in self.cases], name
        self.cases.append({"name": name, "steps": (copy.deepcopy(PRELUDE) if book else []) + steps})


def check_reference():
    """The reference's own geometry: the far objects meet no frame, the edge tables are where the cases say."""
    d = Drawing()
    grounds = [fr["bounds"] for s in d.output()["sheets"] for fr in s["frames"] if "bounds" in fr]
    for i in (2, 3, 4, 5, 6):
        assert not any(meets(box(i), g) for g in grounds), i
    plan = grounds[0]
    assert box(14)[0] == plan[2], "T8 touches the plan's east edge"
    assert abs(box(15)[0] - plan[2] - 0.01) < 1e-12, "T9 a centimetre past it"
    assert meets(box(13), plan) and not VISIBLE["gizli"]


def cases():
    check_reference()
    c = Cases()
    c.add("Koordinat tablosunun nesnesi: tabloyu gösteren pencereler table nedeniyle", [step(Drawing(), [2])])
    c.add("Alan tablosu iki nesneyi okur: pencerede bir kez, girdinin sırasıyla", [step(Drawing(), [3, 2])])
    c.add("Girdinin sırası tersine: tablonun nesneleri de", [step(Drawing(), [2, 3])])
    c.add("Adsız blok çizelgesi her eklemeyi sayar", [step(Drawing(), [5])])
    c.add("Adlı blok çizelgesi yalnız adındaki bloğun eklemesini sayar", [step(Drawing(), [4])])
    c.add("Çizelgenin satır üyesi olan nesne çizelgeyi gösteren pencereyi etkiler", [step(Drawing(), [3])])
    c.add("Hücre alanının okuduğu nesne: gösteren pencerede shows, tablonunkinde table", [step(Drawing(), [1])])
    c.add("Hiçbir tablonun okumadığı uzak nesne yalnız bilinmeyeni etkiler", [step(Drawing(), [6])])
    c.add("Görünüm penceresinde dondurulan katmandaki tablo o pencerede sayılmaz", [step(Drawing(), [3, 1])])
    c.add("Gizli katmandaki tablo hiçbir pencerede table nedeni vermez", [step(Drawing(), [3, 6])])
    c.add("Tablonun kendisi verilince okuduğu nesne onun üzerinden sayılmaz", [step(Drawing(), [7, 2])])
    c.add("Yalnız tablo verilince tablo shows ile, okuduğu yok", [step(Drawing(), [7])])
    c.add("Pencerenin kenarına değen tablo gösterilir (kapalı kutu), 1 cm ötesindeki gösterilmez", [step(Drawing(), [2, 6])])
    c.add("Bütün kaynak türleri birlikte: her pencere bir kez", [step(Drawing(), [1, 2, 3, 4, 5])])
    c.add("Aynı kimlik iki kez: bir kez sayılır", [step(Drawing(), [2, 2, 3])])
    c.add("Plan aynı cevabı verir, çizim değişmez", [step(Drawing(), [2], op="plan")])
    c.add("Doğrula yazmaz, cevap vermez", [step(Drawing(), [2], op="validate")])
    d = Drawing()
    c.add("Silinen tablo pencereyi artık etkilemez; kenardaki öbürü etkiler", [delete(d, 7), step(d, [2])])
    d = Drawing()
    c.add("Silinen çizelge eklemeyi artık saymaz", [delete(d, 9), step(d, [5])])
    d = Drawing()
    c.add("Pencereye sonradan verilen dondurma tabloyu o pencereden çıkarır",
          [freeze_in(d, "a-101", "plan", "tablo"), step(d, [2, 5])])
    d = Drawing()
    c.add("Dondurulan çizim katmanı alan tablosunu değil hücre alanlı tabloyu çıkarır",
          [freeze_in(d, "a-501", "detay", "cizim"), step(d, [1, 3])])
    c.add("Olmayan nesne reddedilir", [refused(Drawing(), [MISSING], 0)])
    c.add("Tablonun okuduğu nesneden sonra olmayan kimlik: yeri uids[1]", [refused(Drawing(), [2, MISSING], 1)])
    c.add("Kitapsız projede tablolu nesneler hiçbir paftayı etkilemez", [step(Drawing(False), [1, 2, 3])], book=False)
    return c


def doc(cs):
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Pafta seti: tabloyu okuyan nesnenin etkisi",
            "note": "scripts/fixtures/sheet_set_table_cases.py yazar; ADR 1090 §6 ve ADR 0347'nin kurallarından, KentOS kodu olmadan.",
            "setup": SETUP, "cases": cs.cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    d = doc(cases())
    assert len(d["cases"]) >= 20, len(d["cases"])
    path = OUT / f"{COMMAND}.table.json"
    text = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND} (table): {len(d['cases'])} durum başvuruyla aynı.")
        return 0
    path.write_text(text)
    print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
