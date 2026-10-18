#!/usr/bin/env python3
"""Pafta seti's shared cases (docs/adr/0347 §1), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.sheets.set.json; `--check` compares and writes nothing. The rules, as the ADR gives them:

- the sheets in the book's order; a sheet's series from its name: its leading ASCII letters upper-cased, an optional `-`,
  `.` or space, and the first digit of the number after them (`A-101` → `A-1`); a name without them `Diğer`; the series
  sorted by their text, `Diğer` last, each with its sheets in the book's order and its title (A-0 Genel, A-1 Planlar,
  A-2 Görünüşler, A-3 Kesitler, A-4 Büyük ölçekli çizimler, A-5 Detaylar; none for another);
- a map frame's ground: a fixed view with a centre only; its content is the frame less its padding and then its label
  band (micrometres on the paper); a content point `p` lies on the ground at `centre + (a, b) / k` where `k` is
  1 000 000 / scale micrometres per metre, `(x, y) = p − content centre`, `a = x·cos + y·sin`, `b = x·sin − y·cos` and
  the angle is the view's turn (here only quarter turns, exact); its corners from the content's top left clockwise, its
  box their extremes; a view mark points at a frame by the sheet and item ids;
- given `uids`: each found (the first missing one `entity_not_found` at `uids[i]`; a repeated id counted once); a frame
  is affected when an object's box meets its ground box (closed boxes; `shows`), when one of the objects is a view mark
  pointing at it (`marked`), or, with any object, when its ground is not known (`unknown`); its objects in the input's
  order. An object's box here is a line's ends, a polygon's corners, a circle's centre ± radius; view marks lie far from
  every frame, so their boxes meet none (the reference asserts the margins).
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as acc  # noqa: E402  (refusals, ids, layers)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
COMMAND = "cad.sheets.set"
P, refusal, layer, uid_of, MISSING = acc.P, acc.refusal, acc.layer, acc.uid_of, acc.MISSING
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
TITLES = {"A-0": "Genel", "A-1": "Planlar", "A-2": "Görünüşler", "A-3": "Kesitler", "A-4": "Büyük ölçekli çizimler",
          "A-5": "Detaylar"}
OTHER = "Diğer"


def mark(form, points, target=None, **kw):
    m = {"form": form, "points": points, **kw}
    if target:
        m["target"] = target
    return m


# The drawing: kind, layer and own fields; the boxes the reference uses for each.
SEED = {
    1: ("line", "cizim", {"a": P(0, 0), "b": P(4, 0)}),
    2: ("polygon", "cizim", {"pts": [P(20, 20), P(24, 20), P(24, 23), P(20, 23)]}),
    3: ("circle", "cizim", {"c": P(10, 0), "r": 1.0}),
    4: ("line", "cizim", {"a": P(500, 500), "b": P(501, 500)}),
    5: ("viewmark", "isaret", mark("section", [P(1000, 0), P(1010, 0)], {"sheet": "a-301", "item": "kesit-a"}, mark="A")),
    6: ("viewmark", "isaret", mark("elevation", [P(1000, 50)], mark="1")),
    7: ("line", "cizim", {"a": P(6.5, 3), "b": P(8, 4)}),
    8: ("line", "cizim", {"a": P(6.625, 0), "b": P(7, 0)}),
    9: ("viewmark", "isaret", mark("detail", [P(1000, 100)], {"sheet": "a-102", "item": "anahtar"}, radius=0.8, mark="2")),
}
IDS = sorted(SEED)
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Pafta seti",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("cizim", "Çizim"), layer("isaret", "İşaret")],
    "activeLayer": "cizim",
    "styles": {"items": [], "categories": []},
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
    else:
        return None
    xs, ys = [p["x"] for p in pts], [p["y"] for p in pts]
    return [min(xs), min(ys), max(xs), max(ys)]


# ── The book ───────────────────────────────────────────────────────────────────────────────────────────────────


def frame(item, name, *, center=None, scale=50, rotation=0, padding=0, band=0, atlas=False, left=20000):
    view = ({"policy": {"scale": 500, "type": "fixed"}, "rotation": 0, "type": "atlas"} if atlas else
            {**({"center": center} if center else {}), "rotation": rotation, "scale": scale, "type": "fixed"})
    return {"bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": 120000, "left": left, "top": 10000, "width": 180000}, "hidden": False, "id": item,
            "kind": {"clipToAtlasFeature": False, "grids": [], "labelBand": band, "layers": {"type": "all"},
                     "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
                     "type": "map", "view": view},
            "locked": False, "name": name, "opacity": 100, "padding": padding, "printable": True, "rotation": 0}


def sheet(id, name, items):
    return {"export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "png"}, "guides": [], "id": id,
            "items": items, "name": name,
            "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                     "paper": "a3", "size": {"height": 297000, "width": 420000}},
            "snapGrid": {"enabled": False, "spacing": 5000, "visible": False}, "variables": [], "variants": []}


# The book's order is not the series' order.
BOOK = [
    sheet("a-301", "A-301 Kesitler", [frame("kesit-a", "Kesit A-A", center=P(100, 100), scale=20, padding=5000, band=3000)]),
    sheet("a-101", "A-101 Zemin kat planı", [frame("plan", "Plan", center=P(2, 0))]),
    sheet("kapak", "Kapak", []),
    sheet("a-102", "a102 1. kat planı", [frame("plan-1", "Plan", center=P(22, 21.5), scale=100, rotation=90000),
                                          frame("anahtar", "Anahtar plan", atlas=True, left=210000)]),
    sheet("a-501", "A.501 Detaylar", [frame("detay-1", "Detay 1", center=P(10, 0), scale=20, rotation=180000),
                                       frame("detay-2", "Detay 2", left=210000)]),
    sheet("s-101", "S 101 Statik plan", [frame("statik", "Statik", center=P(-40, 0), scale=200)]),
]
PRELUDE = [{"op": "execute", "command": "cad.sheets.edit", "note": "pafta kitabı: kurulum kitap veremez",
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


TURNS = {0: (0.0, 1.0), 90000: (1.0, 0.0), 180000: (0.0, -1.0), 270000: (-1.0, 0.0)}


def corners(item):
    view = item["kind"]["view"]
    if view["type"] != "fixed" or "center" not in view:
        return None
    s, co = TURNS[view["rotation"] % 360000]
    f, pad, band = item["frame"], item["padding"], item["kind"]["labelBand"]
    left, top = f["left"] + pad + band, f["top"] + pad + band
    width, height = f["width"] - 2 * (pad + band), f["height"] - 2 * (pad + band)
    rc = (left + width / 2, top + height / 2)
    k = 1_000_000.0 / view["scale"]
    c = view["center"]
    out = []
    for p in [(left, top), (left + width, top), (left + width, top + height), (left, top + height)]:
        x, y = p[0] - rc[0], p[1] - rc[1]
        a, b = x * co + y * s, x * s - y * co
        out.append(P(c["x"] + a / k, c["y"] + b / k))
    return out


def meets(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


class Book:
    def __init__(self, present=True):
        self.sheets = copy.deepcopy(BOOK) if present else []

    def frames(self):
        for s in self.sheets:
            for item in s["items"]:
                yield s, item

    def output(self, uids=None):
        marks = [(i, SEED[i][2].get("target")) for i in IDS if SEED[i][0] == "viewmark"]
        sheets = []
        for s in self.sheets:
            frames = []
            for item in s["items"]:
                c = corners(item)
                view = item["kind"]["view"]
                f = {"item": item["id"], "name": item["name"]}
                if view["type"] == "fixed":
                    f["scale"] = view["scale"]
                if c:
                    f["corners"] = c
                    xs, ys = [p["x"] for p in c], [p["y"] for p in c]
                    f["bounds"] = [min(xs), min(ys), max(xs), max(ys)]
                target = {"sheet": s["id"], "item": item["id"]}
                f["marks"] = [uid_of(i) for i, t in marks if t == target]
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
            ids = []
            for u in uids:
                i = int(u) if u.isdigit() else None
                if i not in ids:
                    ids.append(i)
            affected = []
            for s, f in zip(self.sheets, sheets):
                for item, fr in zip(s["items"], f["frames"]):
                    reasons, objs = [], []
                    if "bounds" in fr:
                        hit = [i for i in ids if box(i) and meets(box(i), fr["bounds"])]
                        for i in ids:  # the view marks' boxes stay far from every frame
                            if SEED[i][0] == "viewmark":
                                x = SEED[i][2]["points"][0]
                                assert abs(x["x"] - fr["bounds"][2]) > 100, (i, fr)
                        if hit:
                            reasons.append("shows")
                            objs += hit
                    target = {"sheet": s["id"], "item": item["id"]}
                    pointing = [i for i in ids if SEED[i][0] == "viewmark" and SEED[i][2].get("target") == target]
                    if pointing:
                        reasons.append("marked")
                        objs += [i for i in pointing if i not in objs]
                    if "bounds" not in fr and ids:
                        reasons.append("unknown")
                    if reasons:
                        objs.sort(key=ids.index)
                        affected.append({"sheet": s["id"], "item": item["id"], "reasons": reasons,
                                         "uids": [uid_of(i) for i in objs]})
            out["affected"] = affected
        out["revision"] = "$current"
        return out


def step(book, uids=None, op="execute"):
    inp = {} if uids is None else {"uids": [uid_of(u) if isinstance(u, int) else u for u in uids]}
    out = book.output(None if uids is None else [str(u) for u in uids])
    return {"op": op, "command": COMMAND, "input": inp,
            "result": {"status": "completed", "output": None if op == "validate" else out, "warnings": []},
            "expect": {"ids": IDS, "revision": "same"}}


def refused(uids, at):
    u = uids[at]
    return {"op": "execute", "command": COMMAND, "input": {"uids": [uid_of(x) if isinstance(x, int) else x for x in uids]},
            "result": refusal("entity_not_found",
                              f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                              f"uids[{at}]"),
            "expect": {"ids": IDS, "revision": "same"}}


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps, book=True):
        assert len(name) > 10, name
        assert name not in [c["name"] for c in self.cases], name
        self.cases.append({"name": name, "steps": (copy.deepcopy(PRELUDE) if book else []) + steps})


def rename(book, sid, to):
    for s in book.sheets:
        if s["id"] == sid:
            s["name"] = to
    return {"op": "execute", "command": "cad.sheets.edit", "input": {"ops": [{"op": "renameSheet", "id": sid, "name": to}]},
            "result": ANY, "expect": {"revision": "changed"}}


def remove(book, sid):
    book.sheets = [s for s in book.sheets if s["id"] != sid]
    return {"op": "execute", "command": "cad.sheets.edit", "input": {"ops": [{"op": "removeSheet", "id": sid}]},
            "result": ANY, "expect": {"revision": "changed"}}


def cases():
    c = Cases()
    c.add("Paftalar kitabın sırasıyla, seriler numaralarından", [step(Book())])
    c.add("Plan aynı cevabı verir, çizim değişmez", [step(Book(), op="plan")])
    c.add("Doğrula yazmaz, cevap vermez", [step(Book(), op="validate")])
    c.add("Plandaki çizgi zemin kat planını etkiler; zemini bilinmeyen çerçeve de", [step(Book(), [1])])
    c.add("Dönük (90°) çerçevenin zemini en ve boyu yer değiştirir", [step(Book(), [2])])
    c.add("180° dönük ayrıntı çerçevesi daireyi gösterir", [step(Book(), [3])])
    c.add("Hiçbir çerçevenin göstermediği nesne yalnız bilinmeyenleri etkiler", [step(Book(), [4])])
    c.add("Görünüm işareti baktığı kesit penceresini etkiler", [step(Book(), [5])])
    c.add("Hedefsiz görünüm işareti hiçbir pencereye bakmaz", [step(Book(), [6])])
    c.add("Atlas penceresine bakan işaret: işaretli ve bilinmeyen", [step(Book(), [9])])
    c.add("Birkaç nesne: her pencere bir kez, nesneleri girdinin sırasıyla", [step(Book(), [1, 2, 3, 5])])
    c.add("Girdinin sırası tersine: pencerelerin nesneleri de", [step(Book(), [5, 3, 2, 1])])
    c.add("Aynı kimlik iki kez: bir kez sayılır", [step(Book(), [1, 1, 3])])
    c.add("Boş kimlik listesi: etkilenen yok, bilinmeyen de", [step(Book(), [])])
    c.add("Çerçevenin köşesine değen nesne gösterilir (kapalı kutu)", [step(Book(), [7])])
    c.add("Kenara 12,5 cm kalan nesne gösterilmez", [step(Book(), [8])])
    c.add("Dolgu ve yazı bandı çerçevenin zeminini daraltır", [step(Book(), [1, 5])])
    c.add("Olmayan nesne reddedilir", [refused([MISSING], 0)])
    c.add("İkinci kimlik olmayınca reddedilir; yeri uids[1]", [refused([1, MISSING], 1)])
    c.add("Kimlik biçiminde olmayan metin olmayan nesnedir", [refused(["yok"], 0)])
    c.add("Kitapsız proje: seri ve pafta yok", [step(Book(False))], book=False)
    c.add("Kitapsız projede nesneler hiçbir paftayı etkilemez", [step(Book(False), [1, 2])], book=False)
    b = Book()
    c.add("Pafta numarasız adla Diğer serisine geçer", [rename(b, "a-101", "Bodrum planı"), step(b)])
    b = Book()
    c.add("Yeniden adlandırılan pafta yeni serisinde", [rename(b, "kapak", "A-001 Kapak"), step(b, [1])])
    b = Book()
    c.add("Silinen pafta setten ve etkilenenlerden çıkar", [remove(b, "a-102"), step(b, [2, 9])])
    return c


def doc(cs):
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1, "title": "Pafta seti",
            "note": "scripts/fixtures/sheet_set_cases.py yazar; ADR 0347'nin kurallarından, KentOS kodu olmadan.",
            "setup": SETUP, "cases": cs.cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    d = doc(cases())
    assert len(d["cases"]) >= 20, len(d["cases"])
    path = OUT / f"{COMMAND}.json"
    text = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(d['cases'])} durum başvuruyla aynı.")
        return 0
    path.write_text(text)
    print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
