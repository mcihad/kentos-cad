#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.labels.report.json: the shared cases of `cad.labels.report` (docs/adr/0533 §6).

The expectation of each case is made here, without KentOS code: the case's drawing becomes a case of the label
engine's independent reference (`label_engine_cases.py`: the engine written again from docs/adr/0212 in Python, its
cases passed by the Rust store), that places the window at the scale, and the counts are read from what it placed:
the states of the labels (placed, unplaced, hidden, pinned, forced), the boxes as drawn (collisions: two placed
boxes that share area; a curved label is not boxed) and the texts (repeats: a placed label whose layer, class and
text a label placed before it has). A part's area comes from the binding's `extent`.

    python3 scripts/fixtures/labels_report_cases.py           # writes the file
    python3 scripts/fixtures/labels_report_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import label_engine_cases as engine  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.labels.report.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
PX_MM = 25.4 / 96.0
X, Y = 500000.0, 4400000.0
ZEROS = "0" * 64

PARCEL = {"placement": "center", "size": 10, "area": "parcel"}
ROAD = {"placement": "along", "size": 10}
POINT = {"placement": "beside", "size": 10, "point": "around"}
WELL = {**POINT, "duplicates": 100.0}
WARN = {"placement": "center", "size": 10, "point": "center", "overlap": "always", "priority": 0}
CROWD = {**POINT, "overlap": "ifNeeded"}


def node(id_, name, label=None, labels=None, point=None, type_="layer", children=None, **extra):
    style = dict(STYLE)
    if label is not None:
        style["label"] = label
    if labels is not None:
        style["labels"] = labels
    if point is not None:
        style["point"] = {"symbol": "ring", "size": point}
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": style, "children": children or [], **extra}


def layers():
    return [
        node("parsel", "Parsel", label=PARCEL),
        node("yol", "Yol", label=ROAD),
        node("nokta", "Nokta", label=POINT, point=6),
        node("kuyu", "Kuyu", label=WELL, point=6),
        node("uyari", "Uyarı", label=WARN, point=6),
        node("kalabalik", "Kalabalık", label=CROWD, point=6),
        node("bina", "Bina", labels={"mode": "off", "obstacle": {"weight": 3}}),
        node("0", "0"),
        node("grup", "Grup", type_="group", children=[node("alt", "Alt")]),
        node("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                                                  "maxZoom": 19}),
    ]


def P(x, y):
    return {"x": x, "y": y}


def rect(x, y, w, h):
    return [P(x, y), P(x + w, y), P(x + w, y + h), P(x, y + h)]


def entities():
    out = []
    n = 0

    def add(e):
        nonlocal n
        n += 1
        out.append({"id": n, "attrs": {}, **e})

    # Six parcels, 40 × 30 m, three across and two deep, numbered.
    for i in range(6):
        add({"kind": "polygon", "layerId": "parsel", "label": str(12 + i), "pts": rect(X + (i % 3) * 40, Y + (i // 3) * 30, 40, 30)})
    # Two streets along the parcels.
    add({"kind": "polyline", "layerId": "yol", "label": "Ankara Yolu", "pts": [P(X, Y - 6), P(X + 120, Y - 4)]})
    add({"kind": "polyline", "layerId": "yol", "label": "Gül Sokak", "pts": [P(X + 120, Y), P(X + 124, Y + 60)]})
    # Six survey points a metre apart: no room for all of their names.
    for i in range(6):
        add({"kind": "point", "layerId": "nokta", "label": f"P.{101 + i}", "p": P(X + 20 + i, Y + 80)})
    # Three wells of one name, ten metres apart: a name once within a hundred pixels.
    for i in range(3):
        add({"kind": "point", "layerId": "kuyu", "label": "Kuyu", "p": P(X + 10 + i * 10, Y + 100)})
    # Two warnings that may be put over another label, on parcels' numbers.
    for i in range(2):
        add({"kind": "point", "layerId": "uyari", "label": "!", "p": P(X + 20 + i * 40, Y + 15)})
    # Six points a metre apart whose names may go over another when no place is free.
    for i in range(6):
        add({"kind": "point", "layerId": "kalabalik", "label": f"K.{i + 1}", "p": P(X + 70 + i, Y + 80)})
    # Buildings, obstacles.
    add({"kind": "polygon", "layerId": "bina", "attrs": {}, "pts": rect(X + 5, Y + 5, 10, 8)})
    add({"kind": "polygon", "layerId": "bina", "attrs": {}, "pts": rect(X + 45, Y + 5, 10, 8)})
    return out


def with_pins(ents, pins):
    ents = copy.deepcopy(ents)
    for e in ents:
        if e["id"] in pins:
            e["labelPins"] = copy.deepcopy(pins[e["id"]])
    return ents


def link(extent=None, layer="parsel", n=1):
    s = {"id": f"kaynak-{n}", "kind": "geopackage", "path": "$fixture:geopackage/v1/qgis.gpkg", "table": "parsel",
         "layer": layer, "revision": "a" * 64, "digest": ZEROS, "srid": 5256}
    if extent is not None:
        s["extent"] = extent
    s["writable"] = True
    return s


def doc(ents=None, tree=None, sources=()):
    d = {
        "format": "kentos.document", "version": 1, "name": "Etiket denetimi",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": P(X, Y),
        "layers": copy.deepcopy(tree if tree is not None else layers()),
        "activeLayer": "parsel",
        "entities": copy.deepcopy(ents if ents is not None else entities()),
        "styles": {"items": [], "categories": []},
    }
    if sources:
        d["settings"]["sources"] = list(sources)
    return d


# ── The reference: the document as a case of the label engine's reference, counted ──

def leaves(tree):
    out = []
    for n in tree:
        if n["type"] == "layer":
            out.append(n)
        out.extend(leaves(n["children"]))
    return out


def engine_case(d, window, scale):
    rows = [n for n in leaves(d["layers"]) if "service" not in n]
    lay, texts, pins = [], [], []
    for i, n in enumerate(rows):
        row = {"id": n["id"], "rank": i}
        if "point" in n["style"]:
            row["point"] = n["style"]["point"]["size"]
        if "label" in n["style"]:
            row["label"] = n["style"]["label"]
        if "labels" in n["style"]:
            row["labels"] = n["style"]["labels"]
        lay.append(row)
    single = {n["id"] for n in rows if "label" in n["style"] and "labels" not in n["style"]}
    objects = []
    for e in d["entities"]:
        o = {k: v for k, v in e.items() if k in ("id", "layerId", "kind", "p", "pts", "holes")}
        objects.append(o)
        if e["layerId"] in single and e.get("label"):
            texts.append({"id": e["id"], "texts": [[0, e["label"]]]})
        if e.get("labelPins"):
            pins.append([e["id"], e["labelPins"]])
    return {"name": "", "window": list(window), "scale": scale, "layers": lay, "objects": objects, "texts": texts,
            "pins": pins, "options": {"unplaced": True, "hidden": True}, "defaults": {}}


def extent_of(d):
    boxes = [engine.bounds(e) for e in d["entities"]]
    if not boxes:
        return None
    return [min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes)]


def counts_of(recs, layer_of):
    """The counts of placed labels read back: states, boxes that share area, repeated texts."""
    placed = [r for r in recs if not r["state"] & (engine.UNPLACED | engine.HIDDEN)]
    boxed = []
    for r in placed:
        if r["state"] & engine.CURVED:
            boxed.append(None)
            continue
        boxed.append(engine.Obb((r["at"][0], r["at"][1]), math.radians(r["angle"]), r["w"] / 2.0, r["h"] / 2.0))
    collides = [False] * len(placed)
    for i, a in enumerate(boxed):
        for j in range(i + 1, len(boxed)):
            b = boxed[j]
            if a is not None and b is not None and a.overlaps(b):
                collides[i] = collides[j] = True
    seen, repeats = set(), [False] * len(placed)
    for i, r in enumerate(placed):
        text = r["text"] if "text" in r else "\n".join(line[4] for line in r["lines"])
        if not text:
            continue
        k = (layer_of[r["id"]], r["class"], text)
        if k in seen:
            repeats[i] = True
        seen.add(k)
    return placed, collides, repeats


def report(d, inp):
    """The answer of `cad.labels.report` for the document and the input, as the reference counts it."""
    rows = [n for n in leaves(d["layers"])]
    scale = inp["pxPerMetre"] if "pxPerMetre" in inp else 1000.0 / (PX_MM * inp["paperScale"])
    window = inp.get("window") or extent_of(d)
    if window is None:
        return {"window": [0.0] * 4, "pxPerMetre": scale, "totals": zero(), "layers": [], "revision": "$current"}
    if not (window[2] > window[0] and window[3] > window[1]):
        window = [window[0] - 1.0, window[1] - 1.0, window[2] + 1.0, window[3] + 1.0]
    case = engine_case(d, window, scale)
    recs = engine.place_window(case)
    layer_of = {e["id"]: e["layerId"] for e in d["entities"]}
    order = {n["id"]: i for i, n in enumerate(rows)}
    # Boxes and repeats over every placed label, in the ground's pixels.
    px = []
    for r in recs:
        q = dict(r)
        q["at"] = (r["at"][0] * scale, r["at"][1] * scale)
        px.append(q)
    placed_px, collides, repeats = counts_of(px, {i: order[layer_of[i]] for i in layer_of})
    wanted = set(inp["layers"]) if inp.get("layers") is not None else None

    # Collisions and repeats by the placed label's place in `placed_px` (the same order as the placed of `px`).
    placed_ix = [i for i, r in enumerate(px) if not r["state"] & (engine.UNPLACED | engine.HIDDEN)]
    col = {placed_ix[k]: collides[k] for k in range(len(placed_ix))}
    rep = {placed_ix[k]: repeats[k] for k in range(len(placed_ix))}

    def counts(select):
        c = zero()
        for i, r in enumerate(px):
            # The window's labels: the engine places a margin round it too.
            at = recs[i]["at"]
            if not (window[0] <= at[0] <= window[2] and window[1] <= at[1] <= window[3]):
                continue
            if not select(r):
                continue
            st = r["state"]
            if st & engine.UNPLACED:
                c["unplaced"] += 1
            elif st & engine.HIDDEN:
                c["hidden"] += 1
            else:
                c["placed"] += 1
                c["pinned"] += 1 if st & engine.PINNED else 0
                c["forced"] += 1 if st & engine.OVERLAPPING else 0
                c["collisions"] += 1 if col[i] else 0
                c["repeats"] += 1 if rep[i] else 0
        return c

    in_scope = lambda r: wanted is None or layer_of[r["id"]] in wanted  # noqa: E731
    out_layers = []
    for n in rows:
        c = counts(lambda r, n=n: layer_of[r["id"]] == n["id"] and in_scope(r))
        src = next((s for s in d["settings"].get("sources", []) if s["layer"] == n["id"]), None)
        # A bound layer with a part, or one that parks pins, is listed with no label too (docs/adr/0578).
        extra = src is not None and ("extent" in src or src.get("parked")) and (wanted is None or n["id"] in wanted)
        if not any(c[k] for k in ("placed", "unplaced", "hidden")) and not extra:
            continue
        row = {"layer": n["id"], "counts": c}
        if src is not None:
            row["source"] = src["id"]
            if "extent" in src:
                e = src["extent"]
                row["part"] = e
                if window[0] < e[0] or window[1] < e[1] or window[2] > e[2] or window[3] > e[3]:
                    row["partial"] = True
                row["band"] = band_of(e)
        out_layers.append(row)
    return {"window": [float(v) for v in window], "pxPerMetre": scale, "totals": counts(in_scope), "layers": out_layers,
            "revision": "$current"}


BAND = 0.15


def band_of(e):
    """The band round a part: its area grown by 15 % of its shorter side on every side (docs/adr/0578 §1)."""
    w = BAND * min(e[2] - e[0], e[3] - e[1])
    return [e[0] - w, e[1] - w, e[2] + w, e[3] + w]


def band_warnings(d, inp):
    """The warnings of the report: a part's band that could not be read. The cases' bindings carry a revision the file
    does not have, so the file is not the version the layer matches."""
    wanted = set(inp["layers"]) if inp.get("layers") is not None else None
    out = []
    for src in d["settings"].get("sources", []):
        if "extent" not in src or (wanted is not None and src["layer"] not in wanted):
            continue
        out.append({"code": "band_unread",
                    "message": f"{src['layer']}: parçanın çevresindeki komşular okunamadı (dosya katmanın son eşleştiği "
                               "sürümden farklı); etiketler yalnız parçanın nesnelerine göre yerleşti.",
                    "path": "layers"})
    return out


def zero():
    return {k: 0 for k in ("placed", "unplaced", "hidden", "pinned", "forced", "collisions", "repeats")}


# ── The cases ─────────────────────────────────────────────────────────────

def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def same_for(d):
    return {"ids": [e["id"] for e in d["entities"]], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def ok(name, inp, d=None, op="execute"):
    d = d if d is not None else doc()
    c = {"name": name, "steps": [{"op": op, "input": inp, "result": completed(report(d, inp), band_warnings(d, inp)), "expect": same_for(d)}]}
    if d is not None:
        c["setup"] = d
    return c


def bad(name, inp, code, message, path, d=None, op="execute"):
    d = d if d is not None else doc()
    return {"name": name, "setup": d,
            "steps": [{"op": op, "input": inp, "result": failed(code, message, path), "expect": same_for(d)}]}


def cases():
    out = []
    base = doc()
    out.append(ok("Çizimin tamamı 1:1000 pafta ölçeğinde: yerleşenler, yerleşmeyenler, yinelenenler", {"paperScale": 1000}))
    ppm = 1000.0 / (PX_MM * 1000)
    out.append(ok("Aynı ölçek piksel bölü metreyle: aynı sayılar", {"pxPerMetre": ppm}))
    out.append(ok("Uzaklaşınca: küçük nesnelerin etiketi yazılmaz", {"pxPerMetre": 0.5}))
    out.append(ok("Yaklaşınca iki parselde: her etiket yerinde", {"pxPerMetre": 20.0, "window": [X, Y, X + 80, Y + 30]}))
    out.append(ok("Pencere yarım: yalnız içindekiler", {"pxPerMetre": 4.0, "window": [X - 10, Y - 10, X + 70, Y + 70]}))
    out.append(ok("Yalnız parsel katmanı sayılır", {"pxPerMetre": 4.0, "layers": ["parsel"]}))
    out.append(ok("Yol ve nokta katmanları sayılır; parsel yine de yer tutar", {"pxPerMetre": 4.0, "layers": ["yol", "nokta"]}))
    out.append(ok("Sık noktalar: adlarının hepsine yer yok", {"pxPerMetre": 4.0, "window": [X + 10, Y + 70, X + 40, Y + 90],
                                                           "layers": ["nokta"]}))
    out.append(ok("Yinelenen ad yakınsa biri yazılır, kuyu katmanı", {"pxPerMetre": 4.0, "window": [X, Y + 90, X + 50, Y + 110],
                                                                  "layers": ["kuyu"]}))
    out.append(ok("Yinelenen ad uzaksa hepsi yazılır: tekrarlananlar sayılır", {"pxPerMetre": 12.0, "window": [X, Y + 90, X + 50, Y + 110],
                                                                              "layers": ["kuyu"]}))
    out.append(ok("Boş yer yoksa bindirebilen sınıfın etiketi üst üste konur: forced", {"pxPerMetre": 4.0, "window": [X + 60, Y + 70, X + 90, Y + 90],
                                                                                      "layers": ["kalabalik"]}))
    out.append(ok("Her zaman bindirebilen sınıf (uyarı) boş yer bulur: kesişen kutu yok",
                  {"pxPerMetre": 4.0, "window": [X, Y, X + 120, Y + 30], "layers": ["uyari", "parsel"]}))
    pinned = with_pins(entities(), {1: [{"at": {"x": -3.0, "y": 8.0}, "rotation": 15.0}]})
    out.append(ok("Sabitlenen etiket yerinde: pinned 1", {"pxPerMetre": 4.0, "window": [X - 10, Y - 10, X + 130, Y + 70]}, doc(pinned)))
    hidden = with_pins(entities(), {1: [{"hidden": True}], 2: [{"hidden": True}]})
    out.append(ok("Elle gizlenen etiketler hidden sayılır", {"pxPerMetre": 4.0, "window": [X - 10, Y - 10, X + 130, Y + 70]},
                  doc(hidden)))
    part = [X - 10, Y - 10, X + 90, Y + 40]
    out.append(ok("Parçası bağlı katman: etiket ve engel yalnız parçanın; pencere parçanın ötesine uzanır, partial",
                  {"pxPerMetre": 4.0}, doc(sources=[link(part)])))
    out.append(ok("Pencere parçanın içindeyse partial yok, part söylenir",
                  {"pxPerMetre": 4.0, "window": [X, Y, X + 60, Y + 30]}, doc(sources=[link(part)])))
    out.append(ok("Bütün tablosu bağlı katman: yalnız kaynağı söylenir", {"pxPerMetre": 4.0}, doc(sources=[link()])))
    out.append(ok("Etiketi kapalı ve yalnız engel katman satır olmaz", {"pxPerMetre": 4.0, "layers": ["bina"]}))
    out.append(ok("Boş çizim: pencere sıfır, sayılar sıfır", {"paperScale": 500}, doc(ents=[])))
    one = [{"id": 1, "attrs": {}, "kind": "point", "layerId": "nokta", "label": "P.1", "p": P(X, Y)}]
    out.append(ok("Tek noktanın kapsamı pencere olmaz: bir metre pay", {"pxPerMetre": 10.0}, doc(ents=one)))
    out.append(ok("plan aynı sayıları verir", {"paperScale": 1000}, op="plan"))
    out.append(bad("Ölçek verilmedi", {}, "invalid_scale", "Ölçek pxPerMetre ya da paperScale'den tam biri verilmeli.", "pxPerMetre"))
    out.append(bad("İki ölçek birden", {"pxPerMetre": 4.0, "paperScale": 1000}, "invalid_scale",
                   "Ölçek pxPerMetre ya da paperScale'den tam biri verilmeli.", "pxPerMetre"))
    out.append(bad("Sıfır piksel bölü metre", {"pxPerMetre": 0.0}, "invalid_scale",
                   "Piksel bölü metre sıfırdan büyük, sonlu bir sayı olmalı.", "pxPerMetre"))
    out.append(bad("Pafta ölçeği sıfır", {"paperScale": 0}, "invalid_scale",
                   "Pafta ölçeği 1 ile 1000000 arasında bir tam sayı olmalı (1:N'nin N'si).", "paperScale"))
    out.append(bad("Pafta ölçeği çok büyük", {"paperScale": 1000001}, "invalid_scale",
                   "Pafta ölçeği 1 ile 1000000 arasında bir tam sayı olmalı (1:N'nin N'si).", "paperScale"))
    out.append(bad("Ters pencere", {"pxPerMetre": 4.0, "window": [X + 10, Y, X, Y + 10]}, "invalid_window",
                   "Pencerenin en küçüğü en büyüğünden küçük olmalı.", "window"))
    out.append(bad("Pencere bu ölçekte çok büyük", {"pxPerMetre": 100.0, "window": [X, Y, X + 20000, Y + 20000]}, "invalid_window",
                   "Pencere bu ölçekte 100 milyon pikselden büyük; daha küçük bir pencere ya da ölçek verin.", "window"))
    out.append(bad("Olmayan katman", {"pxPerMetre": 4.0, "layers": ["parsel", "yok"]}, "layer_not_found",
                   "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layers[1]"))
    out.append(bad("Grup katman değildir", {"pxPerMetre": 4.0, "layers": ["grup"]}, "not_a_layer",
                   "“Grup” bir katman grubu; grubun bir katmanını verin.", "layers[0]"))
    out.append(bad("Servisten çizilen katmanın etiketi olmaz", {"pxPerMetre": 4.0, "layers": ["altlik"]}, "service_layer",
                   "“OpenStreetMap” servisten çizilir ve nesne tutmaz; etiketi olmaz.", "layers[0]"))
    out.append(bad("validate: aynı ret", {"paperScale": 0}, "invalid_scale",
                   "Pafta ölçeği 1 ile 1000000 arasında bir tam sayı olmalı (1:N'nin N'si).", "paperScale", op="validate"))
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.labels.report", "commandVersion": 1,
            "title": "Etiketleri denetle: yerleşen, yerleşmeyen, sabit, bindirilen, kesişen ve yinelenen etiketler",
            "note": "ADR 0533 §6. Etiket motorunun bir pencerede bir ölçekte yerleştirdiklerini sayar; çizimi değiştirmez. "
                    "Bağımsız başvuru: scripts/fixtures/labels_report_cases.py, etiket motorunun Python başvurusuyla "
                    "(label_engine_cases.py), KentOS kodu olmadan.",
            "setup": doc(), "cases": cases()}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/labels_report_cases.py ile yeniden yazın")
            return 1
        print(f"Etiket denetimi durumları tutarlı: {OUT.relative_to(ROOT)}")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(cases())} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
