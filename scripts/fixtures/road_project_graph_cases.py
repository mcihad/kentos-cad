#!/usr/bin/env python3
"""The shared cases of `road.project.graph` (İş grafiği; plan/map-cad.md MAP-20, docs/adr/1940): a road or map
project's chain from its sources to its sheets, read only.

The drawings, their kept records (sources with their stamps, objects written and there now) and the project tree are
road_project_cases.py's (ADR 0911's rules, the stamps by hand); the graph is built from them by hand:

- the layers a surface's points or breaklines name come first, each once, in the surfaces' order (points, then
  breaklines); a layer the drawing lacks is gone;
- then the tree's rows in its order; a drawing carries its recipe: its command, version 1, its input as kept (the main
  source by id), its objects written and there now (the objects' stamp is the command's: `$any`); a drawing a step
  drew keeps the version that drew it (1) and the revision its write made (docs/adr/2840): the revision a
  `captureRevision` step takes right after the step that drew it, or drew it again;
- edges: a surface's layers build it; a row is made on its parent in the tree (not a drawing: its sources say it); every
  source a drawing keeps, in the record's order, the first the main one, its stamp then and now (none: gone; a
  definition's stamp of its grades as KentOS holds them, road_grades.py, as the file is written); a sheet's
  fixed map shows a drawing when its ground (the frame at the map's scale around its centre; no rotation, no padding)
  meets one of the drawing's objects' bounds — a zero line's places are road_terrain_zeroline_cases.py's;
- a sheet that shows none is left out; one that shows a drawing not up to date is stale, the print check's words;
- the project's system by EPSG's registry (5254 “TUREF / TM30”), its epoch and heights as the project says them.

No KentOS code.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_project_graph_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402
import road_project_cases as rp  # noqa: E402
import road_terrain_zeroline_cases as zc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.project.graph.json"
ANY = "$any"
UNCHANGED = rp.UNCHANGED
SAME = {"revision": "same"}
step, completed = rp.step, rp.completed
# EPSG's names of the TUREF systems the setups use.
SYSTEMS = {5253: "TUREF / TM27", 5254: "TUREF / TM30", 5255: "TUREF / TM33", 5256: "TUREF / TM36"}
# A drawing's state in the print check's words (docs/adr/1720 §5).
WORD = {"stale": "güncel değil", "missing": "nesnesi eksik", "edited": "elle değişti", "gone": "kaynağı yok"}
EPOCH = 2005.0
HEIGHTS = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20"}}
HEIGHTS_WORDS = "Ortometrik yükseklik (Antalya height; geoit TG-20)"


# ── The sheets ──────────────────────────────────────────────────────────────────────────────────────────────────────
def page():
    return {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
            "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}}


def map_item(id_, name, x, y, scale=100, w=200000, h=150000):
    """A fixed-view map: its frame in µm on the paper, its centre on the ground and its scale."""
    return {"id": id_, "name": name, "frame": {"left": 20000, "top": 20000, "width": w, "height": h},
            "kind": {"type": "map", "view": {"type": "fixed", "center": {"x": x, "y": y}, "scale": scale, "rotation": 0}}}


def sheet(id_, name, *items):
    return {"id": id_, "name": name, "page": page(), "variants": [], "items": list(items)}


def book(*sheets):
    return {"schema": "kentos.sheet/1", "sheets": list(sheets), "masters": [], "assets": [], "variables": []}


# Pafta 1's Tümsek map is over the mound (20 m × 15 m around it), its other map far away; Pafta 2's map is 30 m east.
BOOK = book(sheet("pafta-1", "Pafta 1", map_item("tumsek", "Tümsek haritası", zc.FX + 5.0, 5.0),
                  map_item("uzak", "Uzak harita", 2500.0, 500.0)),
            sheet("pafta-2", "Pafta 2", map_item("kenar", "Kenar haritası", zc.FX + 40.0, 5.0)))
# Pafta 2's map moved over the mound's east half too (10 m × 7.5 m at 1:50).
BOOK_TWO = book(BOOK["sheets"][0], sheet("pafta-2", "Pafta 2", map_item("dogu", "Doğu haritası", zc.FX + 8.0, 5.0, 50)))


def ground(item):
    f, v = item["frame"], item["kind"]["view"]
    hw, hh = f["width"] * 1e-6 * v["scale"] / 2.0, f["height"] * 1e-6 * v["scale"] / 2.0
    c = v["center"]
    return (c["x"] - hw, c["y"] - hh), (c["x"] + hw, c["y"] + hh)


def meets(box, lo, hi):
    (x0, y0), (x1, y1) = box
    return x1 >= lo[0] and x0 <= hi[0] and y1 >= lo[1] and y0 <= hi[1]


def zero_line_boxes(base="t-5", comparison="t-10"):
    """Each zero line's bounds (road_terrain_zeroline_cases.py's places): one object each."""
    out = []
    for pts, _ in zc.zero_lines(base, comparison)[0]:
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        out.append(((min(xs), min(ys)), (max(xs), max(ys))))
    return out


# ── The stamps as KentOS holds the grades ───────────────────────────────────────────────────────────────────────────
SNAP = {}  # (the state, a drawing's id) → the road settings when it was drawn


def capture(st):
    """A step that takes the revision now (docs/adr/2840), under a name new in the state's case."""
    st.caps = getattr(st, "caps", 0) + 1
    name = f"r{st.caps}"
    return name, {"op": "captureRevision", "as": name}


def drawn(st, what):
    """road_project_cases.py's drawing step and the step that takes the revision its write made, the road settings
    kept as they were when drawn."""
    out = rp.drawn(st, what)
    SNAP[(id(st), st.drawings[-1]["id"])] = copy.deepcopy(st.road)
    name, cap = capture(st)
    st.drawings[-1]["rev"] = name
    return [out, cap]


def redrawn(st):
    """Every drawing's road settings now (after it is drawn again), and the step that takes the revision the drawing
    again made."""
    name, cap = capture(st)
    for d in st.drawings:
        SNAP[(id(st), d["id"])] = copy.deepcopy(st.road)
        d["rev"] = name
    return cap


def stamp_in(st, road, kind, id_):
    """A source's stamp in `road` (road_project_cases.py's rule) with its grades converted as the file holds them; a
    surface's is its sources' digest's (no grade in it)."""
    if kind == "surface":
        return st.stamp_now(kind, id_)
    converted = road_grades.settings(copy.deepcopy(road))
    x = next((x for x in converted.get(rp.LISTS[kind], []) if x["id"] == id_), None)
    return None if x is None else rp.stamp(x)


# ── The graph ───────────────────────────────────────────────────────────────────────────────────────────────────────
def layer_names(layers, out=None):
    out = {} if out is None else out
    for n in layers:
        out[n["id"]] = n["name"]
        layer_names(n.get("children", []), out)
    return out


def layers_of(surface):
    out = []
    for lid in surface.get("points", {}).get("layers", []) + surface.get("breaklines", {}).get("layers", []):
        if lid not in out:
            out.append(lid)
    return out


def graph(st, deep=False, book_=None, boxes=None):
    """The graph of the state `st` (road_project_cases.State), its sheets `book_`'s, its drawings' objects' bounds
    `boxes` (a drawing's id → a box per object there now)."""
    tree = rp.tree(st, deep)
    names = layer_names(st.setup["layers"])
    surfaces = st.road.get("surfaces", [])
    layers = []
    for s in surfaces:
        layers += [lid for lid in layers_of(s) if lid not in layers]
    nodes = []
    for lid in layers:
        if lid in names:
            nodes.append({"kind": "layer", "id": lid, "name": names[lid], "state": "current"})
        else:
            nodes.append({"kind": "layer", "id": lid, "name": lid, "state": "gone", "why": [f"“{lid}” katmanı çizimde yok."]})
    base = len(nodes)
    kept = {d["id"]: d for d in st.drawings}
    for n in tree["nodes"]:
        g = {k: n[k] for k in ("kind", "id", "name", "state")}
        if n.get("why"):
            g["why"] = n["why"]
        if n["kind"] == "drawing":
            d = kept[n["id"]]
            command, input_ = rp.INPUTS[(id(st), d["id"])]
            g["recipe"] = {"command": command, "version": 1, "input": input_, "written": d["n"], "objects": d["present"],
                           "digest": ANY}
            # Docs/adr/2840: the version that drew it and the revision its write made, as its record keeps them.
            if "rev" in d:
                g["recipe"].update({"made": 1, "revision": "$" + d["rev"]})
        nodes.append(g)
    edges = []
    for k, n in enumerate(tree["nodes"]):
        if n["kind"] == "surface":
            s = next(x for x in surfaces if x["id"] == n["id"])
            edges += [{"from": layers.index(lid), "to": base + k, "kind": "builds"} for lid in layers_of(s)]
    for k, n in enumerate(tree["nodes"]):
        if "parent" in n and n["kind"] != "drawing":
            edges.append({"from": base + n["parent"], "to": base + k, "kind": "basis"})
    for k, n in enumerate(tree["nodes"]):
        if n["kind"] != "drawing":
            continue
        for i, (kind, sid, then) in enumerate(kept[n["id"]]["sources"]):
            src = next((j for j, x in enumerate(nodes) if x["kind"] == kind and x["id"] == sid), None)
            if src is None:
                continue
            e = {"from": src, "to": base + k, "kind": "source"}
            if i == 0:
                e["main"] = True
            e["kept"] = then if kind == "surface" else stamp_in(st, SNAP[(id(st), n["id"])], kind, sid)
            now = stamp_in(st, st.road, kind, sid)
            if now is not None:
                e["now"] = now
            edges.append(e)
    for sh in (book_ or {}).get("sheets", []):
        shows = []
        for item in sh["items"]:
            if item["kind"]["type"] != "map" or item["kind"]["view"]["type"] != "fixed":
                continue
            lo, hi = ground(item)
            for k, n in enumerate(tree["nodes"]):
                if n["kind"] == "drawing" and any(meets(b, lo, hi) for b in (boxes or {}).get(n["id"], [])):
                    shows.append((base + k, item["name"]))
        if not shows:
            continue
        why = [f"“{m}” haritası {nodes[k]['name']} çizimini gösteriyor ve çizim {WORD[nodes[k]['state']]}."
               for k, m in shows if nodes[k]["state"] in WORD]
        at = len(nodes)
        node = {"kind": "sheet", "id": sh["id"], "name": sh["name"], "state": "stale" if why else "current"}
        if why:
            node["why"] = why
        nodes.append(node)
        edges += [{"from": k, "to": at, "kind": "shows", "map": m} for k, m in shows]
    count = lambda f: sum(1 for n in nodes if f(n))  # noqa: E731
    settings = st.setup["settings"]
    out = {"nodes": nodes, "edges": edges, "drawings": count(lambda n: n["kind"] == "drawing"),
           "outdated": count(lambda n: n["kind"] == "drawing" and n["state"] in WORD),
           "sheets": count(lambda n: n["kind"] == "sheet"),
           "blocked": count(lambda n: n["kind"] == "sheet" and n["state"] == "stale")}
    srid = settings.get("srid", 0)
    if srid:
        out["epsg"], out["system"] = f"EPSG:{srid}", SYSTEMS[srid]
        if settings.get("coordinateEpoch") is not None:
            out["epoch"] = settings["coordinateEpoch"]
    if settings.get("heights") == HEIGHTS:
        out["heights"] = HEIGHTS_WORDS
    out.update({"deep": deep, "revision": "$current"})
    return out


def asked(st, deep=False, book_=None, boxes=None, op="execute"):
    input_ = {"deep": True} if deep else {}
    out = None if op == "validate" else graph(st, deep, book_, boxes)
    return step(op, input_, completed(out), UNCHANGED)


# ── The setups ──────────────────────────────────────────────────────────────────────────────────────────────────────
def map_setup(book_=BOOK, auto=False, **settings):
    """road_project_cases.py's map project (road_terrain_zeroline_cases.py's lattices) with the sheets."""
    s = rp.map_setup(auto)
    if book_ is not None:
        s["sheets"] = {"book": copy.deepcopy(book_)}
    s["settings"].update(settings)
    return s


def edit_height(st):
    """The mound's corner set lower by an edit of its surface (road_project_cases.py's): the mound's stamp changes."""
    t = st.find("surface", "t-10")
    t["edits"] = [{"op": "setHeight", "x": zc.FX, "y": 0.0, "z": 119.0}]
    return {**step("execute", {"operation": "set", "surface": copy.deepcopy(t)}, rp.ANY), "command": "road.terrains.edit"}


def cases():
    out = []

    def add(name, steps, setup=None):
        if len(steps) > 1:
            steps[-1] = {**steps[-1], "expect": SAME}
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    # A map project.
    sm = map_setup()
    add("Harita projesi, çizim yok: katmanlar yüzeyleri kurar, pafta yok", [asked(rp.State(sm), book_=BOOK)], sm)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    boxes = {"cz-1": zero_line_boxes()}
    add("Sıfır hattı: iki kaynağı damgalarıyla, tarifi ve onu gösteren pafta; uzak harita ve Pafta 2 göstermez",
        [*a, asked(st, book_=BOOK, boxes=boxes)], sm)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    b = edit_height(st)
    add("Tümsek'in bir kotu değişince çizim güncel değil, onu gösteren pafta teslim edilemez",
        [*a, b, asked(st, book_=BOOK, boxes=boxes)], sm)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    b = edit_height(st)
    c = {**rp.rebuilt(st, {}), "command": "road.project.rebuild"}
    cap = redrawn(st)
    add("Yeniden üretilince zincir yine güncel, kaynakların damgaları yeni",
        [*a, b, c, cap, asked(st, book_=BOOK, boxes=boxes)], sm)
    sa = map_setup(auto=True)
    st = rp.State(sa)
    a = drawn(st, rp.ZEROLINE)
    b = edit_height(st)
    rp.rebuild(st, st.drawings[0])
    cap = redrawn(st)
    add("Kendiliğinden yeniden üretimde kot değişince zincir güncel kalır", [*a, b, cap, asked(st, book_=BOOK, boxes=boxes)],
        sa)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    target = len(sm["entities"]) + 1
    st.drawings[0]["present"] -= 1
    b = rp.other("cad.entities.delete", {"uids": [f"$uidOf:{target}"]})
    add("Nesnesi silinen çizim eksik; nesnesi kalmayan çizimi hiçbir pafta göstermez",
        [*a, b, asked(st, book_=BOOK, boxes={"cz-1": []})], sm)
    s2 = map_setup(BOOK_TWO)
    st = rp.State(s2)
    a = drawn(st, rp.ZEROLINE)
    b = edit_height(st)
    add("İki pafta aynı çizimi gösterir: ikisi de teslim edilemez", [*a, b, asked(st, book_=BOOK_TWO, boxes=boxes)], s2)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    add("Plan da sorulur: aynı grafik", [*a, asked(st, book_=BOOK, boxes=boxes, op="plan")], sm)
    st = rp.State(sm)
    add("Doğrulama", [asked(st, book_=BOOK, op="validate")], sm)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    add("Derin okuma: kurulamayan yüzey grafikte de", [*a, asked(st, deep=True, book_=BOOK, boxes=boxes)], sm)
    st = rp.State(sm)
    a = drawn(st, rp.ZEROLINE)
    add("Geri alınan çizim grafikten de gider", [*a, {"op": "undo", "returns": "Sıfır hattı"},
                                                 asked(rp.State(sm), book_=BOOK)], sm)
    se = map_setup(coordinateEpoch=EPOCH, heights=copy.deepcopy(HEIGHTS))
    add("Projenin epoku ve kot sistemi grafikle gelir", [asked(rp.State(se), book_=BOOK)], se)
    # A local project has no system (and no epoch: a .kcad file refuses an epoch without a system).
    sl = map_setup(srid=0)
    add("Yerel projede sistem yok", [asked(rp.State(sl), book_=BOOK)], sl)
    sk = map_setup()
    sk["settings"]["road"]["surfaces"][4]["breaklines"] = {"layers": ["drenaj"]}
    sk["settings"]["road"]["surfaces"].append({"id": "t-11", "name": "Eksik", "points": {"layers": ["yok", "duz"]}})
    add("Kırık çizgi katmanı da yüzeyi kurar; olmayan katman yok", [asked(rp.State(sk), book_=BOOK)], sk)
    # A road project (road_pay_cases.py's drawing).
    s0 = rp.base()
    add("Yol projesi: kaynaklar ve dayanakları, pafta kitabı yok", [asked(rp.State(s0))], s0)
    st = rp.State(s0)
    a, b = drawn(st, rp.CORRIDOR), drawn(st, rp.PROFILE)
    add("Şevli plan ve boykesit çizimi, her kaynağıyla", [*a, *b, asked(st)], s0)
    st = rp.State(s0)
    a = drawn(st, rp.CORRIDOR)
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    b = rp.other("road.templates.edit", {"template": "tk-2", "operation": "set", "name": "Tip 2b"})
    add("Tip kesit değişince yalnız onun kenarı değişir, şevli plan güncel değil", [*a, b, asked(st)], s0)
    st = rp.State(s0)
    a = drawn(st, rp.JUNCTION)
    st.road["junctions"] = [j for j in st.road["junctions"] if j["id"] != "j-1"]
    b = rp.other("road.junctions.edit", {"junction": "j-1", "operation": "remove"})
    add("Kavşak silinince çizimin ana kaynağı yok: kalan kaynaklarının kenarları", [*a, b, asked(st)], s0)
    st = rp.State(s0)
    a = drawn(st, rp.GUARDRAIL)
    st.drawings[0]["edited"] = True
    target = len(s0["entities"]) + 1
    b = rp.other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"})
    add("Elle değişen çizimin tarifi yazıldığı gibi", [*a, b, asked(st)], s0)
    add("Mimari proje değil", [step("execute", {}, rp.NOT_ROAD, UNCHANGED)], rp.base(work="architecture"))
    sg = map_setup(None, workspace="gis")
    sg["settings"].pop("cadWork")
    sg["settings"].pop("road")
    add("CBS projesi değil", [step("execute", {}, rp.NOT_ROAD, UNCHANGED)], sg)
    return out


def file():
    return {"format": "kentos.command-cases", "version": 1, "command": "road.project.graph", "commandVersion": 1,
            "title": "İş grafiği",
            "note": "ADR 1940. scripts/fixtures/road_project_graph_cases.py yazar: çizimler, kayıtları ve proje ağacı "
                    "road_project_cases.py'nin (ADR 0911'in kuralları, damgalar elle); grafik onlardan elle: yüzeylerin "
                    "katmanları, ağacın satırları, kenarlar, paftaların haritalarının zemini (çerçeve × ölçek) ve sıfır "
                    "hattının yerleri road_terrain_zeroline_cases.py'nin. KentOS kodu yok.",
            "setup": map_setup(), "cases": cases()}


def main():
    content = road_grades.fixture(file())
    text = json.dumps(content, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            sys.exit(f"{OUT.name}: güncel değil")
        print(f"{OUT.name}: güncel")
        return
    OUT.write_text(text, "utf-8")
    print(f"{OUT.name}: {len(content['cases'])} durum")


if __name__ == "__main__":
    main()
