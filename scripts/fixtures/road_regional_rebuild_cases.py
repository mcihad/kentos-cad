#!/usr/bin/env python3
"""The shared cases of `road.project.rebuild`'s `regional` (docs/adr/4320; plan/map-cad.md MAP-14): a road drawing drawn
again writing only what changed.

The drawing: a map project's ground, 48 points on a 10 m lattice each moved by a fixed offset (a linear congruential
sequence, millimetres), heights two hills and a hollow on a gentle slope; a design surface of 12 points on its own layer. Each case draws the
ground's contour lines (`road.terrains.contours`, kept as drawing cz-1), changes a source (a height, a place, a point
added or deleted, a breakline) or the drawing by hand, then draws again.

The expectations are made here without KentOS code: the surface is Shewchuk's Triangle's
(road_terrain_command_cases.py's `build`, its triangulation asserted to be the only one of its corners, so KentOS's is
the same), the contour lines are that generator's (`contours`: levels crossing edges as exact fractions, chained and
started at their fixed place). A new line stays where an old one is exactly alike (level, major, closed, places) and not
taken yet; the old ones left are removed, the new ones left written at the drawing's end, in its order. `region` is the
box of the lines removed and written; `ids` is the document's order of objects after the step (kept lines keep their
numbers). Docs/adr/4740: the new lines are all strokes of one look, free in their order; a line painted by hand is
alike none (its colour differs) and is written again alone.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 \\
        scripts/fixtures/road_regional_rebuild_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_command_cases as tc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
INTERVAL = 0.25
NOT_ROAD = tc.failed("not_a_road_project", "Proje ağacı yol ve harita projesinde (KentOS Yol, KentOS Harita) olur; bu çizim "
                                          "ikisinden biri değil.")


def U(n):
    return f"0192f660-7c3e-7d4a-9b1e-{n:012x}"


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


LAYERS = [node("0", "0"), node("zemin", "Zemin"), node("kirik", "Kırık hatlar"), node("proje", "Proje noktaları"),
          node("esyukselti", "Eş yükselti"), node("esyukselti2", "Eş yükselti 2")]


def hill(x, y, cx, cy, s):
    return math.exp(-((x - cx) ** 2 + (y - cy) ** 2) / (2 * s * s))


def lattice():
    """The ground's 48 points (x, y, z), millimetres: two hills and a hollow on a gentle slope, so the lines close round
    them; no height on a quarter metre (no level meets a corner)."""
    state, out = 20261110, []

    def nxt():
        nonlocal state
        state = (1103515245 * state + 12345) % 2 ** 31
        return state

    for j in range(6):
        for i in range(8):
            dx, dy = nxt() % 5001 - 2500, nxt() % 5001 - 2500
            x, y = (10000 * i + dx) / 1000, (10000 * j + dy) / 1000
            mm = round(1000 * (100 + 0.015 * x + 0.01 * y + 2.0 * hill(x, y, 20, 15, 9) + 1.6 * hill(x, y, 52, 36, 8)
                               - 1.0 * hill(x, y, 56, 9, 7)))
            if mm % 250 == 0:
                mm += 1
            out.append((x, y, mm / 1000))
    return out


GROUND = lattice()
DESIGN = [(5.0 + 15 * i, 5.0 + 20 * j, 101.0 + 0.5 * i + 0.25 * j) for j in range(3) for i in range(4)]
# The design's lattice is regular: move each by a little so its triangulation is one.
DESIGN = [(x + 0.137 * k % 1.3, y + 0.071 * k % 0.9, z) for k, (x, y, z) in enumerate(DESIGN)]
T1 = {"id": "t-1", "name": "Mevcut zemin", "points": {"layers": ["zemin"]}, "breaklines": {"layers": ["kirik"]}}
T2 = {"id": "t-2", "name": "Tasarım", "points": {"layers": ["proje"]}}
N0 = len(GROUND) + len(DESIGN)


def setup(work="map"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    # Surfaces are a map or road project's (a file of another project with them does not read).
    if work in ("map", "road"):
        settings["road"] = {"surfaces": [T1, T2]}
    entities = [{"kind": "point", "id": k + 1, "layerId": "zemin", "attrs": {}, "p": {"x": x, "y": y}, "z": z}
                for k, (x, y, z) in enumerate(GROUND)]
    entities += [{"kind": "point", "id": len(GROUND) + k + 1, "layerId": "proje", "attrs": {}, "p": {"x": x, "y": y},
                  "z": z} for k, (x, y, z) in enumerate(DESIGN)]
    return {"format": "kentos.document", "version": 2, "name": "Bölgesel yeniden üretim", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
            "entities": entities, "uids": [U(k + 1) for k in range(N0)], "styles": {"items": [], "categories": []}}


class State:
    """The document as the steps leave it: its objects' numbers in order, the next number, the sources by layer and
    the drawings' lines (number, key, places)."""

    def __init__(self):
        self.ids = list(range(1, N0 + 1))
        self.next = N0 + 1
        # Each source object: (number, kind, layer, geometry as road_terrain_command_cases.py reads it).
        self.sources = [(k + 1, "point", "zemin", {"p": tc.P(x, y), "z": z}) for k, (x, y, z) in enumerate(GROUND)]
        self.sources += [(len(GROUND) + k + 1, "point", "proje", {"p": tc.P(x, y), "z": z})
                         for k, (x, y, z) in enumerate(DESIGN)]
        self.drawings = {}

    def lines(self, surface, interval=INTERVAL, major=5):
        """The surface's contour lines now (road_terrain_command_cases.py's surface and lines)."""
        tc.ALL_ENTITIES = [(k, l, g) for _, k, l, g in self.sources]
        tc.ALL_UIDS = [U(n) for n, _, _, _ in self.sources]
        tc.PAD_LAYERS = LAYERS
        g, left = tc.gather(surface)
        assert not left, left
        s = tc.build(g)
        tc.one_triangulation(s, g)
        return tc.contours(s, interval, 0.0, major)

    def draw(self, name, surface, interval=INTERVAL):
        made = self.lines(surface, interval)
        self.drawings[name] = (surface, interval, [(self.take(), key(l), l["points"]) for l in made])
        return len(made)

    def take(self):
        n = self.next
        self.next += 1
        self.ids.append(n)
        return n

    def source(self, n):
        return next(s for s in self.sources if s[0] == n)

    def set_source(self, n, geo):
        self.sources = [(m, k, l, geo if m == n else g) for m, k, l, g in self.sources]

    def remove(self, numbers):
        self.ids = [i for i in self.ids if i not in numbers]
        self.sources = [s for s in self.sources if s[0] not in numbers]
        for name, (surface, interval, lines) in self.drawings.items():
            self.drawings[name] = (surface, interval, [x for x in lines if x[0] not in numbers])

    def redraw(self, name, regional=True, colours=None):
        """Draws `name` again (the module's rules); answers its `rebuilt` row."""
        surface, interval, old = self.drawings[name]
        made = self.lines(surface, interval)
        new_keys = [key(l) for l in made]
        # Docs/adr/4740: a line painted by hand is alike no new one (every new line has the drawing's look).
        painted = {n for n, c in (colours or {}).items() if c is not None}
        stays = [None] * len(made)
        if regional:
            free = [i for i in range(len(old)) if old[i][0] not in painted]
            for j, k in enumerate(new_keys):
                hit = next((i for i in free if old[i][1] == k), None)
                if hit is not None:
                    stays[j] = hit
                    free.remove(hit)
        taken = {i for i in stays if i is not None}
        gone = [old[i] for i in range(len(old)) if i not in taken]
        self.ids = [i for i in self.ids if i not in {g[0] for g in gone}]
        lines = []
        written = []
        for j, l in enumerate(made):
            if stays[j] is None:
                n = self.take()
                written.append(l["points"])
                lines.append((n, new_keys[j], l["points"]))
            else:
                lines.append(old[stays[j]])
        self.drawings[name] = (surface, interval, lines)
        row = {"drawing": name, "command": "road.terrains.contours", "removed": len(gone)}
        if regional:
            row["kept"] = len(made) - len(written)
            places = [p for _, _, pts in gone for p in pts] + [p for pts in written for p in pts]
            if places:
                row["region"] = [min(p[0] for p in places), min(p[1] for p in places),
                                 max(p[0] for p in places), max(p[1] for p in places)]
        row["uids"] = "$any"
        return row


def key(l):
    return (l["z"], l["major"], l["closed"], tuple(tuple(p) for p in l["points"]))


def point(x, y, z):
    return {"kind": "point", "p": {"x": x, "y": y}, "z": z}


def contours_step(st, name="cz-1", surface=T1, layer="esyukselti", interval=INTERVAL):
    st.draw(name, surface, interval)
    return {"op": "execute", "command": "road.terrains.contours",
            "input": {"surface": surface["id"], "interval": interval, "layerId": layer}, "result": ANY, "within": W}


def height_step(st, n, z):
    _, _, _, g = st.source(n)
    st.set_source(n, {**g, "z": z})
    return {"op": "execute", "command": "cad.entities.edit",
            "input": {"operation": "elevation", "changes": [{"kind": "update", "uid": f"$uidOf:{n}",
                                                              "geometry": point(g["p"]["x"], g["p"]["y"], z)}]},
            "result": ANY, "within": W}


def rebuild_step(rows, input_=None, expect=None, op="execute", left=None):
    output = {"rebuilt": rows, "revision": "$current"}
    if left:
        output["left"] = left
    if op == "plan":
        output["rebuilt"] = [{**r, "uids": []} for r in rows]
    result = tc.completed(output if op != "validate" else None)
    s = {"op": op, "input": {"regional": True} if input_ is None else input_, "result": result, "within": W}
    if expect is not None:
        s["expect"] = expect
    return s


def changed(st):
    return {"ids": list(st.ids), "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}


def inner(st, near):
    """The ground point nearest `near` (x, y)."""
    return min((s for s in st.sources if s[2] == "zemin"),
               key=lambda s: (s[3]["p"]["x"] - near[0]) ** 2 + (s[3]["p"]["y"] - near[1]) ** 2)[0]


def quiet_height(st, surface=T1):
    """A ground point and a height for it under which no contour line changes (the drawing comes out alike)."""
    before = [key(l) for l in st.lines(surface)]
    for n, kind, layer, g in st.sources:
        if layer != "zemin":
            continue
        for dz in (0.001, -0.001, 0.002, -0.002, 0.003, -0.003):
            z = round(g["z"] + dz, 3)
            st.set_source(n, {**g, "z": z})
            alike = [key(l) for l in st.lines(surface)] == before
            st.set_source(n, g)
            if alike:
                return n, z
    raise AssertionError("no quiet height")


def cases():
    out = []

    def case(name, steps, note=None, setup_=None):
        c = {"name": name, **({"note": note} if note else {}), "steps": steps}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    # 1. A height changed: only the lines round the point are written again.
    st = State()
    s1 = contours_step(st)
    n = inner(st, (52, 36))
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    row = st.redraw("cz-1")
    assert 0 < row["kept"] < len(st.drawings["cz-1"][2]), row
    case("Bir noktanın kotu değişince yalnız çevresindeki eğriler yeniden yazılır, öbürleri yerinde",
         [s1, s2, rebuild_step([row], expect=changed(st))],
         note="kept: aynı çıkan çizgiler numaralarıyla yerinde; region silinen ve yazılan çizgilerin kutusu")

    # 2. The same without regional: every line is written again.
    st = State()
    s1 = contours_step(st)
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    row = st.redraw("cz-1", regional=False)
    case("Aynı değişiklik regional olmadan: bütün eğriler yeniden yazılır",
         [s1, s2, rebuild_step([row], input_={}, expect=changed(st))])

    # 3–4. Plan and validation write nothing.
    st = State()
    s1 = contours_step(st)
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    before = list(st.ids)
    row = st.redraw("cz-1")
    case("Plan: yazmadan söyler (yerinde kalan, silinen, bölge)",
         [s1, s2, rebuild_step([row], op="plan", expect={"ids": before, "revision": "same"})])
    case("Doğrulama: yazmaz",
         [s1, s2, rebuild_step([row], op="validate", expect={"ids": before, "revision": "same"})])

    # 5–6. Undo puts the old lines back where they were; redo writes the same again.
    st = State()
    s1 = contours_step(st)
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    before = list(st.ids)
    row = st.redraw("cz-1")
    after = list(st.ids)
    case("Geri alınınca eski eğriler yerinde, çizim yine eskimiş; yinelenince yine yalnız değişenler",
         [s1, s2, rebuild_step([row], expect=changed(st)),
          {"op": "undo", "returns": "Yeniden üret", "expect": {"ids": before, "canRedo": True, "revision": "changed"}},
          {"op": "redo", "returns": "Yeniden üret", "expect": {"ids": after, "canRedo": False, "revision": "changed"}},
          {"op": "execute", "input": {"regional": True}, "result": tc.completed({"rebuilt": [], "revision": "$current"}),
           "within": W, "expect": {"ids": after, "revision": "same"}}])

    # 7. A point moved (its place, not its height).
    st = State()
    s1 = contours_step(st)
    m = inner(st, (22, 12))
    _, _, _, g = st.source(m)
    moved = {"p": tc.P(round(g["p"]["x"] + 1.25, 3), round(g["p"]["y"] - 0.75, 3)), "z": g["z"]}
    st.set_source(m, moved)
    s2 = {"op": "execute", "command": "cad.entities.edit",
          "input": {"operation": "grip", "changes": [{"kind": "update", "uid": f"$uidOf:{m}",
                                                       "geometry": point(moved["p"]["x"], moved["p"]["y"], g["z"])}]},
          "result": ANY, "within": W}
    row = st.redraw("cz-1")
    assert row["kept"] > 0
    case("Nokta kaydırılınca yalnız çevresi", [s1, s2, rebuild_step([row], expect=changed(st))])

    # 8. A point added.
    st = State()
    s1 = contours_step(st)
    new = st.take()
    st.sources.append((new, "point", "zemin", {"p": tc.P(47.25, 33.5), "z": 104.9}))
    s2 = {"op": "execute", "command": "cad.entities.create",
          "input": {"layerId": "zemin", "objects": [{"geometry": point(47.25, 33.5, 104.9)}]}, "result": ANY,
          "within": W}
    row = st.redraw("cz-1")
    assert row["kept"] > 0
    case("Yeni nokta eklenince yalnız çevresi; yeni nokta eğrilerin arasında kalır",
         [s1, s2, rebuild_step([row], expect=changed(st))])

    # 9. A point deleted.
    st = State()
    s1 = contours_step(st)
    gone = inner(st, (52, 38))
    st.remove({gone})
    s2 = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uidOf:{gone}"]}, "result": ANY,
          "within": W}
    row = st.redraw("cz-1")
    assert row["kept"] > 0
    case("Nokta silinince yalnız çevresi", [s1, s2, rebuild_step([row], expect=changed(st))])

    # 10. A change no line follows: nothing is written, the record takes the stamps.
    st = State()
    s1 = contours_step(st)
    q, qz = quiet_height(st)
    s2 = height_step(st, q, qz)
    row = st.redraw("cz-1")
    assert row["removed"] == 0 and "region" not in row, row
    case("Eğrileri değiştirmeyen kot değişikliği: hiçbir nesne yazılmaz, çizim güncel olur",
         [s1, s2, rebuild_step([row], expect=changed(st)),
          {"op": "execute", "input": {"regional": True}, "result": tc.completed({"rebuilt": [], "revision": "$current"}),
           "within": W, "expect": {"ids": list(st.ids), "revision": "same"}}],
         note="kaydın kaynak damgası yenilenir (geri alınabilir değişiklik); nesneler numaralarıyla aynı")

    # 11. A current drawing named: nothing written, nothing changed.
    st = State()
    s1 = contours_step(st)
    row = st.redraw("cz-1")
    case("Güncel çizim adıyla regional ile istenince hiçbir şey yazılmaz",
         [s1, rebuild_step([row], input_={"drawings": ["cz-1"], "regional": True},
                           expect={"ids": list(st.ids), "revision": "same"})])

    # 12. A line deleted by hand: only it is written again.
    st = State()
    s1 = contours_step(st)
    lost = st.drawings["cz-1"][2][3][0]
    st.remove({lost})
    s2 = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uidOf:{lost}"]}, "result": ANY,
          "within": W}
    row = st.redraw("cz-1")
    assert row["removed"] == 0 and row["kept"] == len(st.drawings["cz-1"][2]) - 1, row
    case("Elle silinen eğri yeniden yazılır, öbürleri yerinde (nesnesi eksik çizim)",
         [s1, s2, rebuild_step([row], expect=changed(st))])

    # 13. A line changed by hand, drawn again by name with edited: only it.
    st = State()
    s1 = contours_step(st)
    num, k_, pts = st.drawings["cz-1"][2][5]
    bent = [list(p) for p in pts]
    bent[0] = [bent[0][0] + 0.5, bent[0][1] + 0.5]
    lines = st.drawings["cz-1"][2]
    st.drawings["cz-1"] = (T1, INTERVAL, [(a, ("elle",), bent) if a == num else (a, b, c) for a, b, c in lines])
    s2 = {"op": "execute", "command": "cad.entities.edit",
          "input": {"operation": "grip", "changes": [{"kind": "update", "uid": f"$uidOf:{num}", "geometry": {
              "kind": "polyline", "pts": [{"x": x, "y": y} for x, y in bent], "zs": [k_[0]] * len(bent),
              "closed": k_[2]}}]}, "result": ANY, "within": W}
    row = st.redraw("cz-1")
    assert row["removed"] == 1, row
    case("Elle değişen eğri edited ile: yalnız o yeniden yazılır",
         [s1, s2, rebuild_step([row], input_={"drawings": ["cz-1"], "edited": True, "regional": True},
                               expect=changed(st))])

    # 14. A line coloured by hand (docs/adr/4740): it alone is written again in the drawing's look.
    st = State()
    s1 = contours_step(st)
    num = st.drawings["cz-1"][2][2][0]
    colours = {a: None for a, _, _ in st.drawings["cz-1"][2]}
    colours[num] = "#E5484D"
    s2 = {"op": "execute", "command": "cad.entities.set",
          "input": {"uids": [f"$uidOf:{num}"], "color": "#E5484D", "operation": "color"}, "result": ANY, "within": W}
    row = st.redraw("cz-1", colours=colours)
    assert row["removed"] == 1 and row["kept"] == len(st.drawings["cz-1"][2]) - 1 and "region" in row, row
    case("Elle rengi değişen eğri: yalnız o çizimin rengiyle yeniden yazılır, öbürleri yerinde",
         [s1, s2, rebuild_step([row], input_={"drawings": ["cz-1"], "edited": True, "regional": True},
                               expect=changed(st))],
         note="ADR 4740: yeni eğrilerin hepsi tek renkte çizgi, sıraları resmi değiştirmez; boyalı eski eğri hiçbirine eş değil")

    # 15. Two drawings of two surfaces: only the stale one.
    st = State()
    s1 = contours_step(st)
    s2 = contours_step(st, "cz-2", T2, "esyukselti2")
    s3 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    row = st.redraw("cz-1")
    case("İki yüzeyin iki çizimi: yalnız eskiyen yeniden üretilir",
         [s1, s2, s3, rebuild_step([row], expect=changed(st))])

    # 16. Two drawings of one surface (two intervals): both, each only where it changed.
    st = State()
    s1 = contours_step(st)
    s2 = contours_step(st, "cz-2", T1, "esyukselti2", 1.0)
    s3 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    rows = [st.redraw("cz-1"), st.redraw("cz-2")]
    case("Aynı yüzeyin iki aralıkta iki çizimi: ikisi de yalnız değiştiği yerde",
         [s1, s2, s3, rebuild_step(rows, expect=changed(st))])

    # 17. A breakline added.
    st = State()
    s1 = contours_step(st)
    bl = st.take()
    ridge = [(18.4, 21.7, 101.9), (31.3, 27.9, 103.4)]
    st.sources.append((bl, "polyline", "kirik", {"pts": [tc.P(x, y) for x, y, _ in ridge],
                                                   "zs": [z for _, _, z in ridge]}))
    s2 = {"op": "execute", "command": "cad.entities.create",
          "input": {"layerId": "kirik", "objects": [{"geometry": {
              "kind": "polyline", "pts": [{"x": x, "y": y} for x, y, _ in ridge], "zs": [z for _, _, z in ridge],
              "closed": False}}]}, "result": ANY, "within": W}
    row = st.redraw("cz-1")
    assert row["kept"] > 0
    case("Kırık hat eklenince yalnız boyunca", [s1, s2, rebuild_step([row], expect=changed(st))])

    # 18. Every height raised: no line is alike, none stays.
    st = State()
    s1 = contours_step(st)
    changes = []
    for num, kind, layer, g in list(st.sources):
        if layer == "zemin":
            z = round(g["z"] + 0.07, 3)
            st.set_source(num, {**g, "z": z})
            changes.append({"kind": "update", "uid": f"$uidOf:{num}", "geometry": point(g["p"]["x"], g["p"]["y"], z)})
    s2 = {"op": "execute", "command": "cad.entities.edit", "input": {"operation": "elevation", "changes": changes},
          "result": ANY, "within": W}
    row = st.redraw("cz-1")
    assert row["kept"] == 0 and "region" in row
    case("Bütün kotlar yükselince hiçbir eğri aynı değil: hepsi yeniden yazılır, bölge bütün çizim",
         [s1, s2, rebuild_step([row], expect=changed(st))])

    # 19. A locked layer: refused, nothing written.
    st = State()
    s1 = contours_step(st)
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    before = list(st.ids)
    lock = {"op": "execute", "command": "cad.layers.set",
            "input": {"operation": "locked", "layer": "esyukselti", "value": True}, "result": ANY, "within": W}
    case("Kilitlenen katmandaki çizim yeniden üretilemez, hiçbir şey yazılmaz",
         [s1, s2, lock, {"op": "execute", "input": {"regional": True}, "result": tc.failed(
             "layer_locked", "“Eş yükselti” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
             "etkinleştirin.", "layerId"), "within": W, "expect": {"ids": before, "revision": "same"}}])

    # 20. The surface no longer builds: refused.
    st = State()
    s1 = contours_step(st)
    x0, y0 = st.source(1)[3]["p"]["x"], st.source(1)[3]["p"]["y"]
    twin = {"op": "execute", "command": "cad.entities.create",
            "input": {"layerId": "zemin", "objects": [{"geometry": point(x0, y0, st.source(1)[3]["z"] + 1.0)}]},
            "result": ANY, "within": W}
    before = list(st.ids) + [st.next]
    case("Kaynağı kurulamayan yüzeyin çizimi reddedilir, hiçbir şey yazılmaz",
         [s1, twin, {"op": "execute", "input": {"regional": True}, "result": tc.failed(
             "terrain_unbuildable", "Kaynaklardan yüzey kurulamıyor: bir yerde iki farklı kot var "
             f"(yer: {tc.rust_float(x0)}, {tc.rust_float(y0)})."), "within": W,
             "expect": {"ids": before, "revision": "same"}}])

    # 21. The surface removed: the drawing is left, gone.
    st = State()
    s1 = contours_step(st)
    drop = {"op": "execute", "command": "road.terrains.edit", "input": {"operation": "remove", "id": "t-1"},
            "result": ANY, "within": W}
    case("Kaynağı silinen çizim hepsinde kalır", [s1, drop, rebuild_step(
        [], left=[{"drawing": "cz-1", "why": "gone", "message": "Kaynağı projede yok."}],
        expect={"ids": list(st.ids), "revision": "same"})])

    # 22. A road project, as a map project.
    st = State()
    s1 = contours_step(st)
    s2 = height_step(st, n, round(st.source(n)[3]["z"] + 0.6, 3))
    row = st.redraw("cz-1")
    case("Yol projesinde de yalnız değişenler", [s1, s2, rebuild_step([row], expect=changed(st))],
         setup_=setup("road"))

    # 23. Not a road or map project.
    case("Yol ve harita projesi değil", [{"op": "execute", "input": {"regional": True}, "result": NOT_ROAD,
                                          "within": W, "expect": {"revision": "same"}}], setup_=setup("architecture"))

    # 24. The expected revision.
    case("Beklenen revizyon tutmuyor", [{"op": "execute", "input": {"regional": True, "expectedRevision": "41"},
                                         "result": tc.CONFLICT, "within": W, "expect": {"revision": "same"}}])
    return out


def files():
    yield "road.project.rebuild.regional.json", {
        "format": "kentos.command-cases", "version": 1, "command": "road.project.rebuild", "commandVersion": 1,
        "title": "Yalnız değişeni yeniden üret (ADR 4320, plan/map-cad.md MAP-14)",
        "note": "scripts/fixtures/road_regional_rebuild_cases.py yazar: yüzey Triangle'ın (üçgenlemesinin tek olduğu "
                "doğrulanır), eğriler road_terrain_command_cases.py'nin kesirli hesabı; yeni çizgi tıpatıp aynı eski "
                "çizginin yerinde kalır, kalan eskiler silinir, kalan yeniler çizimin sırasıyla sona yazılır. "
                "KentOS kodu yok.",
        "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv
    bad = False
    for name, data in files():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{name}: güncel değil")
                bad = True
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{name}: {len(data['cases'])} durum")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
