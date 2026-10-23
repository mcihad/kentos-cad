"""The shared cases of Çakışma and Topoloji as a command's input (`rules` of cad.polygon.create and the shape commands)
and of cad.entities.adjoin (Bitişik alan), docs/adr/0162, 0622 §9.

    python3 scripts/fixtures/area_rules_cases.py           # writes the files
    python3 scripts/fixtures/area_rules_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.polygon.create.area-rules.json, cad.entities.adjoin.json and cad.parcels.create.json. Every area here has straight
edges along east and north on whole metres, so the reference works on the unit cells of the plane, without KentOS
code and without a single rounding:

- an area is the set of cells whose centres are inside it; Çakışma takes away the cells of the shown areas of the
  avoided layers (a hidden layer's are not shown), counting the areas that shared a cell with it; none left is a
  refusal;
- the result's corners are the cell corners where one or three of the four cells round it are in, or two across,
  and the corners of the areas it was cut along (or, Bitişik alan, closed with) lying on its boundary: a shared
  boundary keeps the neighbours' corners; its parts the 4-connected groups of cells, its holes the groups of empty
  cells the outside does not reach;
- Topoloji: a corner of the new area strictly inside a neighbour's edge goes to that neighbour (a locked one is
  counted and takes none), a neighbour's corner strictly inside the new area's edge, not yet one of its corners,
  goes to the new area; a point
  is a neighbour only with `points`;
- Parsel oluştur: the parcel goes on the `parsel` layer (opened as a new project has it, “Parsel sınırı”, when the
  drawing lacks it, in the parcel's step), its number the given one or one above the greatest whole number its
  Parsel values start with (JavaScript's parseInt), its attributes Ada, Parsel, Mahalle, Nitelik “Arsa”, Tapu alanı
  (m²) and Pafta, the given ones over them, its own six kept even when empty, other empty ones not written, its label the
  number; a number another parcel has is warned of;
- Bitişik alan: the cells neither in a neighbour nor reachable from the far outside without crossing the path or a
  neighbour's edge are the region; the path closes none when there are none.

The runner compares an area's shape whatever corner its rings start from (`shapes`: the sorted corners, the net
area, the parts and the holes). The messages, codes and paths are the ADR's.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000

# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────


def layer(id_, name, visible=True, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


def area_entity(id_, layer_id, ring):
    return {"kind": "polygon", "id": id_, "layerId": layer_id, "attrs": {}, "pts": [W(p) for p in ring]}


# The neighbours: two parcels side by side, a road, a locked area, a hidden one, a point and a second road piece.
NEIGHBOURS = {
    1: ("parsel", rect(0, 0, 10, 10)),
    2: ("parsel", rect(10, 0, 20, 10)),
    3: ("yol", rect(30, 0, 40, 20)),
    4: ("kilitli", rect(50, 0, 60, 10)),
    5: ("gizli", rect(0, 30, 20, 40)),
    6: ("yol", rect(12, 15, 18, 25)),
}
POINT = (7, "parsel", (80, 5))
LAYERS = [l for l in POLYGON["setup"]["layers"]] + [layer("parsel", "Parsel"), layer("yol", "Yol")]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k not in ("layers", "entities")},
         "layers": LAYERS,
         "entities": [area_entity(i, l, r) for i, (l, r) in NEIGHBOURS.items()]
         + [{"kind": "point", "id": POINT[0], "layerId": POINT[1], "attrs": {}, "p": W(POINT[2])}]}
IDS = [e["id"] for e in SETUP["entities"]]
NEXT = max(IDS) + 1
SHOWN = {i for i, (l, _) in NEIGHBOURS.items() if l != "gizli"}
LOCKED = {"kilitli"}
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}

# ── Cells ────────────────────────────────────────────────────────────────────────────────────────────────────────


def inside(ring, cx, cy):
    """Even-odd test of a cell centre (half-integers: never on an edge)."""
    n, c = len(ring), False
    for i in range(n):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % n]
        if (y0 > cy) != (y1 > cy) and cx < x0 + (cy - y0) * (x1 - x0) / (y1 - y0):
            c = not c
    return c


def cells(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return {(x, y) for x in range(min(xs), max(xs)) for y in range(min(ys), max(ys)) if inside(ring, x + 0.5, y + 0.5)}


def corners(cs):
    out = set()
    xs = [c[0] for c in cs]
    ys = [c[1] for c in cs]
    for x in range(min(xs), max(xs) + 2):
        for y in range(min(ys), max(ys) + 2):
            a, b, c, d = (x - 1, y - 1) in cs, (x, y - 1) in cs, (x - 1, y) in cs, (x, y) in cs
            k = a + b + c + d
            if k in (1, 3) or (k == 2 and a == d):
                out.add((x, y))
    return out


def groups(cs):
    left, out = set(cs), []
    while left:
        seed = left.pop()
        group, todo = {seed}, [seed]
        while todo:
            x, y = todo.pop()
            for q in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if q in left:
                    left.discard(q)
                    group.add(q)
                    todo.append(q)
        out.append(group)
    return out


def holes(cs):
    xs = [c[0] for c in cs]
    ys = [c[1] for c in cs]
    box = {(x, y) for x in range(min(xs) - 1, max(xs) + 2) for y in range(min(ys) - 1, max(ys) + 2)}
    empty = box - cs
    return [g for g in groups(empty) if not any(c[0] in (min(xs) - 1, max(xs) + 1) or c[1] in (min(ys) - 1, max(ys) + 1)
                                                 for c in g)]


def on_edge(ring, p):
    """Whether p lies strictly inside one of the ring's edges (not at its corners)."""
    n = len(ring)
    for i in range(n):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % n]
        if p in (ring[i], ring[(i + 1) % n]):
            return False
    for i in range(n):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % n]
        if (x1 - x0) * (p[1] - y0) == (y1 - y0) * (p[0] - x0) and min(x0, x1) <= p[0] <= max(x0, x1) \
                and min(y0, y1) <= p[1] <= max(y0, y1):
            return True
    return False


def boundary_edges(cs):
    """The unit edges between a cell in and one out, as pairs of corners."""
    out = []
    for (x, y) in cs:
        for (dx, dy), e in (((0, -1), ((x, y), (x + 1, y))), ((0, 1), ((x, y + 1), (x + 1, y + 1))),
                            ((-1, 0), ((x, y), (x, y + 1))), ((1, 0), ((x + 1, y), (x + 1, y + 1)))):
            if (x + dx, y + dy) not in cs:
                out.append(e)
    return out


def on_boundary(cs, p, own_corners):
    """Whether p lies on the cells' boundary and is not one of its corners."""
    if p in own_corners:
        return False
    return any(p in e for e in boundary_edges(cs))


def summary(cs, extra=()):
    cn = sorted(corners(cs) | set(extra))
    return {"corners": [[X + x, Y + y] for x, y in cn], "area": len(cs), "parts": len(groups(cs)),
            "holes": len(holes(cs))}


def ring_summary(ring, extra=()):
    """A neighbour's shape: its corners and those it was given."""
    return {**summary(cells(ring)), "corners": [[X + x, Y + y] for x, y in sorted(set(ring) | set(extra))]}


# ── The rules ────────────────────────────────────────────────────────────────────────────────────────────────────


def ruled(ring, avoid=(), topology=False, points=False):
    """What the rules make of a new area: its cells, the corners it takes, the neighbours' new corners, the counts."""
    cs = cells(ring)
    cut = []
    for i, (l, r) in NEIGHBOURS.items():
        if l in avoid and i in SHOWN:
            n = cells(r)
            if cs & n:
                cut.append(r)
                cs = cs - n
    overlapped = len(cut)
    if overlapped and not cs:
        return None
    kept = kept_corners(cs, cut)
    taken, given, edited, locked = set(), {}, set(), 0
    if topology:
        own = corners(cs) | kept
        for i, (l, r) in NEIGHBOURS.items():
            if i not in SHOWN:
                continue
            gets = [c for c in sorted(own) if on_edge(r, c)]
            if gets:
                if l in LOCKED:
                    locked += 1
                else:
                    given[i] = gets
                    edited.add(i)
            for c in r:
                if on_boundary(cs, c, own):
                    taken.add(c)
        if points and on_boundary(cs, POINT[2], own):
            taken.add(POINT[2])
    return {"cells": cs, "overlapped": overlapped, "taken": taken, "given": given, "locked": locked, "kept": kept}


def kept_corners(cs, rings):
    """The corners of the areas a boundary was cut along (or closed with) lying on it: kept as its own."""
    own = corners(cs)
    return {c for r in rings for c in r if on_boundary(cs, c, own)}


def warnings_of(r):
    out = []
    if r["overlapped"]:
        out.append({"code": "area_clipped", "message": f"Çakışma önlendi: {r['overlapped']} komşu alanla örtüşen kısım çıkarıldı.",
                    "path": "rules.avoid"})
    if r["given"]:
        n = sum(len(v) for v in r["given"].values())
        out.append({"code": "corners_joined",
                    "message": f"Topolojik düzenleme: {len(r['given'])} komşu nesneye yeni alanın {n} köşesi eklendi.",
                    "path": "rules.topology"})
    if r["taken"]:
        out.append({"code": "corners_joined", "message": f"Topolojik düzenleme: yeni alana komşulardan {len(r['taken'])} köşe eklendi.",
                    "path": "rules.topology"})
    if r["locked"]:
        out.append({"code": "neighbours_locked", "message": f"Kilitli katmandaki {r['locked']} komşu nesneye köşe eklenmedi.",
                    "path": "rules.topology"})
    return out


def neighbour_shapes(r):
    return {str(i): ring_summary(NEIGHBOURS[i][1], gets) for i, gets in r["given"].items()}


def rules_of(avoid=(), topology=False, points=False):
    out = {}
    if avoid:
        out["avoid"] = list(avoid)
    if topology:
        out["topology"] = True
    if points:
        out["points"] = True
    return out


def poly_input(ring, rules=None, layer_id="yapi"):
    inp = {"layerId": layer_id, "pts": [W(p) for p in ring]}
    if rules is not None:
        inp["rules"] = rules
    return inp


def written_case(name, ring, avoid=(), topology=False, points=False, command=None, inp=None, undo="Ekle"):
    r = ruled(ring, avoid, topology, points)
    assert r is not None, name
    rules = rules_of(avoid, topology, points)
    step = {"op": "execute", "input": inp if inp is not None else poly_input(ring, rules)}
    if command:
        step["command"] = command
    step["result"] = {"status": "completed", "output": {"uid": "$uid", "id": NEXT, "revision": "$current"},
                      "warnings": warnings_of(r)}
    step["expect"] = {"ids": IDS + [NEXT], "shapes": {str(NEXT): summary(r["cells"], r["taken"] | r["kept"]),
                                                      **neighbour_shapes(r)},
                      "canUndo": True, "dirty": True}
    back = {"op": "undo", "returns": undo,
            "expect": {"ids": IDS, "canUndo": False, "canRedo": True,
                       "shapes": {str(i): ring_summary(NEIGHBOURS[i][1]) for i in r["given"]}}}
    if not r["given"]:
        del back["expect"]["shapes"]
    return {"name": name, "steps": [step, back]}


def refused(name, inp, code, message, path, command=None, non_finite=None):
    step = {"op": "execute", "input": inp}
    if command:
        step["command"] = command
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


COVERED = ("Yeni alan kaçınılan katmanlardaki alanların içinde kalıyor; yazılacak bir kısmı yok. Başka bir yere çizin "
           "ya da Çakışma'yı kapatın.")


def polygon_cases():
    sq = rect(5, 5, 15, 15)
    out = [
        written_case("Kuralsız: komşuyla örtüşen alan olduğu gibi yazılır", sq),
        written_case("Boş kurallar hiçbir şeyi değiştirmez", sq),
        written_case("Çakışma: iki parselle örtüşen kısım çıkarılır", sq, avoid=["parsel"]),
        written_case("Çakışma: yalnız seçili katman (Yol) keser, parsel kesmez", rect(25, 5, 35, 15), avoid=["yol"]),
        written_case("Çakışma: iki katmandan kaçınılır", rect(5, 5, 35, 12), avoid=["parsel", "yol"]),
        written_case("Çakışma: gizli katmanın alanı kesmez", rect(5, 35, 15, 45), avoid=["gizli"]),
        written_case("Çakışma: kilitli katmanın alanı da keser", rect(55, 5, 65, 15), avoid=["kilitli"]),
        written_case("Çakışma: kalan iki parça tek nesne", rect(-5, 4, 25, 6), avoid=["parsel"]),
        written_case("Çakışma: komşuları saran alanda delik kalır", rect(-5, -5, 25, 15), avoid=["parsel"]),
        written_case("Topoloji: köşeler komşuların kenarına, iki komşu", rect(20, 2, 30, 8), topology=True),
        written_case("Topoloji: komşuların köşeleri yeni alanın kenarına", rect(20, -5, 30, 15), topology=True),
        written_case("Topoloji: kilitli komşu köşe almaz, söylenir", rect(60, 2, 70, 8), topology=True),
        written_case("Topoloji ve Noktalar da: nokta yeni alanın köşesi olur", rect(80, 0, 90, 10), topology=True, points=True),
        written_case("Topoloji, Noktalar da kapalı: nokta köşe olmaz", rect(80, 0, 90, 10), topology=True),
        written_case("Noktalar da Topolojisiz bir şey değiştirmez", rect(80, 0, 90, 10), points=True),
        written_case("Çakışma ve Topoloji birlikte", sq, avoid=["parsel"], topology=True),
        written_case("Dikdörtgen komutunda Çakışma", sq, avoid=["parsel"], command="cad.rectangle.create",
                     inp={"layerId": "yapi", "frame": {"kind": "corners", "a": W((5, 5)), "b": W((15, 15))},
                          "rules": {"avoid": ["parsel"]}}),
        written_case("Dikdörtgen komutunda Topoloji", rect(20, 2, 30, 8), topology=True, command="cad.rectangle.create",
                     inp={"layerId": "yapi", "frame": {"kind": "corners", "a": W((20, 2)), "b": W((30, 8))},
                          "rules": {"topology": True}}),
        written_case("Dikdörtgen komutunda kilitli komşu", rect(60, 2, 70, 8), topology=True, command="cad.rectangle.create",
                     inp={"layerId": "yapi", "frame": {"kind": "corners", "a": W((60, 2)), "b": W((70, 8))},
                          "rules": {"topology": True}}),
    ]
    # The empty rules' case gives `rules: {}`.
    out[1]["steps"][0]["input"]["rules"] = {}
    r = ruled(sq, ["parsel"])
    out.append({"name": "Plan: kırpılan alan, yazılmaz", "steps": [
        {"op": "plan", "input": poly_input(sq, {"avoid": ["parsel"]}),
         "result": {"status": "completed", "output": {"entity": "$any", "revision": "$current"}, "warnings": warnings_of(r)},
         "expect": UNCHANGED}]})
    out += [
        refused("Çakışma: komşuların içinde kalan alan yazılmaz", poly_input(rect(2, 2, 8, 8), {"avoid": ["parsel"]}),
                "area_covered", COVERED, "rules.avoid"),
        refused("Kaçınılan katman yok", poly_input(sq, {"avoid": ["yok"]}), "layer_not_found",
                "Kaçınılacak “yok” katmanı çizimde yok. Var olan bir katmanın kimliğini verin.", "rules.avoid[0]"),
        refused("Kaçınılan düğüm grup", poly_input(sq, {"avoid": ["parsel", "pafta"]}), "not_a_layer",
                "Kaçınılacak “Pafta” bir grup; yalnız katmanların alanlarından kaçınılır.", "rules.avoid[1]"),
        refused("Dikdörtgen komutunda komşuların içinde", None, "area_covered", COVERED, "rules.avoid",
                command="cad.rectangle.create"),
    ]
    out[-1]["steps"][0]["input"] = {"layerId": "yapi", "frame": {"kind": "corners", "a": W((2, 2)), "b": W((8, 8))},
                                    "rules": {"avoid": ["parsel"]}}
    return out


# ── Bitişik alan ─────────────────────────────────────────────────────────────────────────────────────────────────


def path_walls(path):
    walls = set()
    for (x0, y0), (x1, y1) in zip(path, path[1:]):
        if x0 == x1:
            for y in range(min(y0, y1), max(y0, y1)):
                walls.add(((x0, y), (x0, y + 1)))
        else:
            for x in range(min(x0, x1), max(x0, x1)):
                walls.add(((x, y0), (x + 1, y0)))
    return walls


def region(path, layers, within=None):
    """The cells the path closes with the shown areas of `layers` (those touching `within`, when given)."""
    used = []
    for i, (l, r) in NEIGHBOURS.items():
        if l in layers and i in SHOWN:
            if within:
                xs, ys = [p[0] for p in r], [p[1] for p in r]
                (a, b), (c, d) = within
                if max(xs) < a or min(xs) > c or max(ys) < b or min(ys) > d:
                    continue
            used.append(cells(r))
    taken = set().union(*used) if used else set()
    walls = path_walls(path)
    for cs in used:
        for e in boundary_edges(cs):
            walls.add(e)
    pts = path + [p for (_, r) in NEIGHBOURS.items() for p in r[1]]
    lo_x, hi_x = min(p[0] for p in pts) - 2, max(p[0] for p in pts) + 2
    lo_y, hi_y = min(p[1] for p in pts) - 2, max(p[1] for p in pts) + 2

    def wall(a, b):
        return (a, b) in walls or (b, a) in walls

    start = (lo_x, lo_y)
    seen, todo = {start}, [start]
    while todo:
        x, y = todo.pop()
        for (nx, ny), e in (((x + 1, y), ((x + 1, y), (x + 1, y + 1))), ((x - 1, y), ((x, y), (x, y + 1))),
                            ((x, y + 1), ((x, y + 1), (x + 1, y + 1))), ((x, y - 1), ((x, y), (x + 1, y)))):
            if lo_x <= nx < hi_x and lo_y <= ny < hi_y and (nx, ny) not in seen and (nx, ny) not in taken and not wall(*e):
                seen.add((nx, ny))
                todo.append((nx, ny))
    return {(x, y) for x in range(lo_x, hi_x) for y in range(lo_y, hi_y)} - seen - taken


NO_REGION = "Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun."
ADJOIN_PATH = [(5, 5), (5, 20), (15, 20), (15, 5)]


def adjoin_input(path, layer_id="yapi", **extra):
    return {"layerId": layer_id, "pts": [W(p) for p in path], **extra}


def used_rings(layers, within=None):
    out = []
    for i, (l, r) in NEIGHBOURS.items():
        if l in layers and i in SHOWN:
            if within:
                xs, ys = [p[0] for p in r], [p[1] for p in r]
                (a, b), (c, d) = within
                if max(xs) < a or min(xs) > c or max(ys) < b or min(ys) > d:
                    continue
            out.append(r)
    return out


def adjoin_written(name, path, layers=("parsel",), within=None, avoid=(), topology=False, inp=None):
    cs = region(path, layers, within)
    assert cs, name
    cut = used_rings(layers, within)
    overlapped = 0
    for i, (l, r) in NEIGHBOURS.items():
        if l in avoid and i in SHOWN and cs & cells(r):
            overlapped += 1
            cut.append(r)
            cs = cs - cells(r)
    kept = kept_corners(cs, cut)
    taken, given = set(), {}
    if topology:
        own = corners(cs) | kept
        for i, (l, r) in NEIGHBOURS.items():
            if i not in SHOWN:
                continue
            gets = [c for c in sorted(own) if on_edge(r, c)]
            if gets and l not in LOCKED:
                given[i] = gets
            for c in r:
                if on_boundary(cs, c, own):
                    taken.add(c)
    res = {"cells": cs, "overlapped": overlapped, "taken": taken, "given": given, "locked": 0}
    step = {"op": "execute", "input": inp or adjoin_input(path, neighbours=list(layers))}
    step["result"] = {"status": "completed", "output": {"uid": "$uid", "id": NEXT, "revision": "$current"},
                      "warnings": warnings_of(res)}
    step["expect"] = {"ids": IDS + [NEXT], "shapes": {str(NEXT): summary(cs, taken | kept), **neighbour_shapes(res)},
                      "canUndo": True, "dirty": True}
    back = {"op": "undo", "returns": "Bitişik alan", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def adjoin_cases():
    p = ADJOIN_PATH
    out = [
        adjoin_written("İki parselin üstünü kapatan yol; adım “Bitişik alan”", p),
        adjoin_written("Komşular verilmezse alanın kendi katmanı", p, inp=adjoin_input(p, "parsel")),
        adjoin_written("Daha alçak yol", [(5, 5), (5, 15), (15, 15), (15, 5)]),
        adjoin_written("Kutu iki parseli de kapsar", p, within=((0, 0), (30, 30)),
                       inp=adjoin_input(p, neighbours=["parsel"], within={"min": W((0, 0)), "max": W((30, 30))})),
        adjoin_written("Kenar başına sıfır bükümler düz yoldur", p,
                       inp=adjoin_input(p, neighbours=["parsel"], bulges=[0, 0, 0])),
        adjoin_written("Çakışma: Yol katmanının alanı bölgeden çıkarılır", p, avoid=["yol"],
                       inp=adjoin_input(p, neighbours=["parsel"], rules={"avoid": ["yol"]})),
        adjoin_written("Topoloji: bölge parsellerle köşe köşe bağlanır", p, topology=True,
                       inp=adjoin_input(p, neighbours=["parsel"], rules={"topology": True})),
        adjoin_written("Renk, kalınlık, öznitelik ve etiket", p,
                       inp=adjoin_input(p, neighbours=["parsel"], color="#E5484D", lineWeight=0.5, attrs={"Tür": "Ek"},
                                        label="B")),
    ]
    out[-1]["steps"][0]["expect"]["fields"] = {str(NEXT): {"color": "#E5484D", "lineWeight": 0.5, "attrs": {"Tür": "Ek"},
                                                           "label": "B"}}
    cs = region(p, ["parsel"])
    assert cs
    out.append({"name": "Plan: bölge olacağı gibi, yazılmaz", "steps": [
        {"op": "plan", "input": adjoin_input(p, neighbours=["parsel"]),
         "result": {"status": "completed", "output": {"entity": "$any", "revision": "$current"}, "warnings": []},
         "expect": UNCHANGED}]})
    assert not region([(5, 5), (5, 20), (25, 20)], ["parsel"])
    assert not region(p, ["parsel"], within=((0, 0), (9, 30)))
    assert not region(p, ["gizli"])
    poly = POLYGON_ERRORS
    out += [
        refused("Yol kapanmıyor", adjoin_input([(5, 5), (5, 20), (25, 20)], neighbours=["parsel"]), "no_region", NO_REGION, "pts"),
        refused("Kutu ikinci parseli dışarıda bırakır: yol kapanmaz", adjoin_input(
            p, neighbours=["parsel"], within={"min": W((0, 0)), "max": W((9, 30))}), "no_region", NO_REGION, "pts"),
        refused("Gizli katmanın alanları komşu değildir", adjoin_input(p, neighbours=["gizli"]), "no_region", NO_REGION, "pts"),
        refused("Tek nokta", adjoin_input([(5, 5)]), "too_few_points",
                "Bitişik alanın yolunun en az iki noktası olmalı; 1 nokta verildi.", "pts"),
        refused("Nokta sonlu değil", adjoin_input(p), "not_finite",
                "2. noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "pts[1].x", non_finite={"pts[1].x": "NaN"}),
        refused("Büküm sayısı kenar sayısı değil", adjoin_input(p, bulges=[0, 0]), "bulge_count",
                "Yay değerleri kenar sayısı kadar olmalı: 3 kenar, 2 yay değeri verildi. Her kenara bir değer verin; düz kenar için 0.",
                "bulges"),
        refused("Büküm sonlu değil", adjoin_input(p, bulges=[0, 0, 0]), "not_finite",
                "2. kenarın yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.",
                "bulges[1]", non_finite={"bulges[1]": "Infinity"}),
        refused("Komşu katman yok", adjoin_input(p, neighbours=["yok"]), "layer_not_found",
                "Komşu “yok” katmanı çizimde yok. Var olan bir katmanın kimliğini verin.", "neighbours[0]"),
        refused("Komşu düğüm grup", adjoin_input(p, neighbours=["parsel", "pafta"]), "not_a_layer",
                "Komşu “Pafta” bir grup; bölge yalnız katmanların alanlarıyla kapanır.", "neighbours[1]"),
        refused("Kilitli katmana yazılmaz", adjoin_input(p, "kilitli", neighbours=["parsel"]), "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId"),
        refused("Olmayan katmana yazılmaz", adjoin_input(p, "yok", neighbours=["parsel"]), "layer_not_found",
                poly["layer_not_found"]["message"], "layerId"),
        refused("Kalınlık 100 mm'den büyük", adjoin_input(p, neighbours=["parsel"], lineWeight=101), "invalid_line_weight",
                poly["invalid_line_weight"]["message"], "lineWeight"),
        refused("Beklenen sürüm sayı değil", adjoin_input(p, neighbours=["parsel"], expectedRevision="012"), "invalid_revision",
                poly["invalid_revision"]["message"], "expectedRevision"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": adjoin_input(p, neighbours=["parsel"]),
             "result": {"status": "completed", "output": {"uid": "$uid", "id": NEXT, "revision": "$current"}, "warnings": []}},
            {"op": "execute", "input": adjoin_input(p, neighbours=["parsel"], expectedRevision="$old"),
             "result": {"status": "conflict", "error": {"code": "revision_conflict",
                                                        "message": poly["revision_conflict"]["message"],
                                                        "path": "expectedRevision", "revision": "$current"}}}]},
    ]
    hidden = adjoin_written("Gizli katmana uyarıyla", p, inp=adjoin_input(p, "gizli", neighbours=["parsel"]))
    hidden["steps"][0]["result"]["warnings"] = [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.",
                                                 "path": "layerId"}]
    out.append(hidden)
    return out


# ── Parsel oluştur ───────────────────────────────────────────────────────────────────────────────────────────────

PARCEL_ATTRS = ["Ada", "Parsel", "Mahalle", "Nitelik", "Tapu alanı (m²)", "Pafta"]


def parcel_setup(with_layer=True):
    out = json.loads(json.dumps(SETUP))
    for e in out["entities"]:
        if e["id"] == 1:
            e["attrs"] = {"Ada": "101", "Parsel": "7"}
        if e["id"] == 2:
            e["attrs"] = {"Ada": "101", "Parsel": " 12/A"}
    if not with_layer:
        out["layers"] = [l for l in out["layers"] if l["id"] != "parsel"]
        out["entities"] = [e for e in out["entities"] if e["layerId"] != "parsel"]
    return out


def js_parse_int(text):
    rest = text.lstrip(" \t\n\r\ufeff").removeprefix("+")
    digits = ""
    for ch in rest:
        if not ch.isdigit() or not ch.isascii():
            break
        digits += ch
    return int(digits) if digits else None


def next_number(setup):
    nums = [js_parse_int(e["attrs"].get("Parsel", "")) or 0 for e in setup["entities"]
            if e["layerId"] == "parsel" and "Parsel" in e["attrs"]]
    return str(max(nums, default=0) + 1)


def parcel_attrs(number, given=None):
    """The parcel's attributes as the drawing keeps them: its own six always, empty when not given (Öznitelikler
    offers them to fill), the other given ones canonical, the empty ones not written."""
    out = {k: "" for k in PARCEL_ATTRS}
    out["Nitelik"] = "Arsa"
    out.update(given or {})
    out["Parsel"] = number
    return {k: v for k, v in out.items() if v != "" or k in PARCEL_ATTRS}


def parcel_written(name, ring, number=None, attrs=None, avoid=(), topology=False, setup=None, warnings=(), extra=None):
    setup = setup or parcel_setup()
    ids = [e["id"] for e in setup["entities"]]
    nid = max(ids) + 1
    num = number if number is not None else next_number(setup)
    inp = {"pts": [W(p) for p in ring]}
    if number is not None:
        inp["number"] = number
    if attrs:
        inp["attrs"] = attrs
    rules = rules_of(avoid, topology)
    if rules:
        inp["rules"] = rules
    inp.update(extra or {})
    r = ruled(ring, avoid, topology)
    assert r is not None, name
    opened = not any(l["id"] == "parsel" for l in setup["layers"])
    expect = {"ids": ids + [nid], "shapes": {str(nid): summary(r["cells"], r["taken"] | r["kept"])},
              "fields": {str(nid): {"layerId": "parsel", "attrs": parcel_attrs(num, attrs), "label": num,
                                    **({k: v for k, v in (extra or {}).items() if k in ("color", "lineWeight")})}},
              "canUndo": True, "dirty": True}
    if opened:
        expect["nodes"] = {"parsel": {"name": "Parsel sınırı", "type": "layer", "parent": None}}
    case = {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": {"status": "completed", "output": {"uid": "$uid", "id": nid, "number": num, "opened": opened,
                                                      "revision": "$current"},
                    "warnings": list(warnings) + warnings_of(r)},
         "expect": expect},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": ids, "canUndo": False, "canRedo": True}}]}
    if setup is not SETUP_PARCEL:
        case["setup"] = setup
    return case


SETUP_PARCEL = parcel_setup()
TAKEN = "“{n}” numaralı başka bir parsel var; numara yine verildi. Parsel numarasını Öznitelikler'den düzeltebilirsiniz."


def parcel_cases():
    sq = rect(60, 20, 70, 30)
    out = [
        parcel_written("Sıradaki numara: başındaki tam sayıların en büyüğünün bir fazlası (12/A'dan 13)", sq,
                       setup=SETUP_PARCEL),
        parcel_written("Verilen numara", sq, number="101/5", setup=SETUP_PARCEL),
        parcel_written("Öznitelikler varsayılanların üstüne, Parsel numaradır", sq, number="20",
                       attrs={"Ada": "102", "Mahalle": "Merkez", "Parsel": "99"}, setup=SETUP_PARCEL),
        parcel_written("Başka parselin numarası uyarıyla verilir", sq, number="7", setup=SETUP_PARCEL,
                       warnings=[{"code": "parcel_number_taken", "message": TAKEN.format(n="7"), "path": "number"}]),
        parcel_written("Parsel katmanı yoksa aynı adımda açılır, numara 1", sq, setup=parcel_setup(False)),
        parcel_written("Çakışma: parsellerle örtüşen kısım çıkarılır", rect(5, 5, 15, 15), avoid=["parsel"],
                       setup=SETUP_PARCEL),
        parcel_written("Topoloji: komşu parsellere köşe", rect(20, 2, 30, 8), topology=True, setup=SETUP_PARCEL),
        parcel_written("Renk ve kalınlık", sq, setup=SETUP_PARCEL, extra={"color": "#E5484D", "lineWeight": 0.35}),
        parcel_written("Nitelik verilir", sq, attrs={"Nitelik": "Tarla", "Pafta": "G21-b"}, setup=SETUP_PARCEL),
        parcel_written("Çakışma ve Topoloji birlikte", rect(5, 5, 15, 15), avoid=["parsel"], topology=True,
                       setup=SETUP_PARCEL),
        parcel_written("Numaranın başı ve sonundaki boşluk kırpılır", sq, number="15", setup=SETUP_PARCEL,
                       extra={"number": "  15 "}),
    ]
    # The neighbours' new corners in the topology cases.
    r = ruled(rect(20, 2, 30, 8), (), True)
    out[6]["steps"][0]["expect"]["shapes"].update(neighbour_shapes(r))
    r = ruled(rect(5, 5, 15, 15), ("parsel",), True)
    out[9]["steps"][0]["expect"]["shapes"].update(neighbour_shapes(r))
    out.append({"name": "Plan: yazılmaz, numarası söylenir", "steps": [
        {"op": "plan", "input": {"pts": [W(p) for p in sq]},
         "result": {"status": "completed", "output": {"entity": "$any", "number": "13", "opened": False,
                                                      "revision": "$current"}, "warnings": []},
         "expect": {"ids": [e["id"] for e in SETUP_PARCEL["entities"]], "dirty": False, "revision": "same"}}]})
    out.append({"name": "Plan: katman yoksa açılacağı söylenir, çizim değişmez", "setup": parcel_setup(False), "steps": [
        {"op": "plan", "input": {"pts": [W(p) for p in sq]},
         "result": {"status": "completed", "output": {"entity": "$any", "number": "1", "opened": True,
                                                      "revision": "$current"}, "warnings": []},
         "expect": {"ids": [e["id"] for e in parcel_setup(False)["entities"]], "dirty": False, "revision": "same"}}]})
    locked = parcel_setup()
    for l in locked["layers"]:
        if l["id"] == "parsel":
            l["locked"] = True
    poly = POLYGON_ERRORS
    unchanged = {"ids": [e["id"] for e in SETUP_PARCEL["entities"]], "dirty": False, "revision": "same"}

    def no(name, inp, code, message, path, non_finite=None, setup=None):
        step = {"op": "execute", "input": inp}
        if non_finite:
            step["nonFinite"] = non_finite
        error = {"code": code, "message": message}
        if path:
            error["path"] = path
        step["result"] = {"status": "failed", "error": error}
        step["expect"] = unchanged if setup is None else {"dirty": False, "revision": "same"}
        case = {"name": name, "steps": [step]}
        if setup:
            case["setup"] = setup
        return case

    base = {"pts": [W(p) for p in sq]}
    out += [
        no("Boş numara", {**base, "number": "  "}, "invalid_parcel_number",
           "Parsel numarası boş olamaz. Bir numara verin ya da sıradakini almak için numarayı vermeyin.", "number"),
        no("Kilitli parsel katmanı", base, "layer_locked",
           "“Parsel” katmanı kilitli; Parsel bu katmana yazar. Kilidi Katmanlar panelinden açın.", None,
           setup=locked),
        no("İki köşe", {"pts": [W(p) for p in sq[:2]]}, "too_few_corners",
           "Kapalı alanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "pts"),
        no("Köşe sonlu değil", base, "not_finite",
           "2. köşenin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
           "pts[1].x", non_finite={"pts[1].x": "NaN"}),
        no("Çakışma: parsellerin içinde kalan alan", {"pts": [W(p) for p in rect(2, 2, 8, 8)], "rules": {"avoid": ["parsel"]}},
           "area_covered", COVERED, "rules.avoid"),
        no("Kalınlık 100 mm'den büyük", {**base, "lineWeight": 101}, "invalid_line_weight",
           poly["invalid_line_weight"]["message"], "lineWeight"),
        no("Beklenen sürüm sayı değil", {**base, "expectedRevision": "012"}, "invalid_revision",
           poly["invalid_revision"]["message"], "expectedRevision"),
        no("Kaçınılan katman yok", {**base, "rules": {"avoid": ["yok"]}}, "layer_not_found",
           "Kaçınılacak “yok” katmanı çizimde yok. Var olan bir katmanın kimliğini verin.", "rules.avoid[0]"),
    ]
    return out


POLYGON_ERRORS = {}
for case in POLYGON["cases"]:
    for step in case["steps"]:
        e = step.get("result", {}).get("error")
        if e:
            POLYGON_ERRORS.setdefault(e["code"], e)

FILES = [
    ("cad.polygon.create.area-rules", "cad.polygon.create", "Kapalı alan: Çakışma ve Topoloji", polygon_cases),
    ("cad.entities.adjoin", "cad.entities.adjoin", "Bitişik alan", adjoin_cases),
    ("cad.parcels.create", "cad.parcels.create", "Parsel oluştur", parcel_cases),
]


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for name, command, title, cases in FILES:
        doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
               "note": "Written by scripts/fixtures/area_rules_cases.py on the unit cells of the plane (docs/adr/0162, "
                       "0622 §9); do not edit by hand.",
               "setup": SETUP_PARCEL if command == "cad.parcels.create" else SETUP, "cases": cases()}
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / f"{name}.json"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = True
            else:
                print(f"{name} durumları güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
