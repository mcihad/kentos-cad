#!/usr/bin/env python3
"""cad.arch.walk v1 (“Dolaşım güzergâhı”): the shared cases, written from the command's rules without KentOS code.

The walls of these drawings run along x or y, so a floor's edges, a passage's mouth and a column's sides are exact
sums; the walking room's rounded corners are shapely's (GEOS) buffers, a route around them its shortest polyline.

The rules:
- The input: the start and the end finite (not_finite at `start`, then `end`: “Çizimde sonlu bir konum seçin.”), the
  walking width 0.05–10 m both included (invalid_input at `width`: “Geçiş genişliği 0,05–10 m arasında olmalı.”), a
  level the project has (level_not_found at `level`: “Çizimde bulunan bir kat seçin.”). Nothing is ever written.
- The floor: the spaces of the level asked (none asked: the spaces without a level), voids and broken spaces left
  out, each as drawn (a space given by its rings is its own net part). A start or an end outside it is where the route
  is blocked: no points, no length.
- More than 256 edges of the floor's rings (then with the obstacles' outlines too) are refused: invalid_input without a
  path, “Bu katta rota hesabı için çok fazla sınır var. Daha küçük bir çizim kapsamı kullanın.”
- A passage joins the floor: a door (its clear width: a swinging leaf open at 90° takes its 4 cm thickness from the
  light) or an opening without a sill (its own width), a rectangle that wide on the wall's middle, across the wall and
  15 mm into the floor on both sides; it joins only when both of those ends are floor.
- Out of the floor: the level's columns and, unless `shellOnly`, its blocks whose card's first category is Mobilya,
  Mutfak or Islak hacim, by the box of what they draw.
- The walking room is the floor shrunk by half the width and a millimetre. When one piece of it holds both ends, the
  route is the shortest polyline in that piece from the start to the end: its points (a straight one: the two ends)
  and its length. Otherwise the route is blocked where the straight line from the start first leaves the piece that
  holds the start (the start itself when none does), and the obstacle is the first column or block, in the drawing's
  order, whose outline holds that point or comes within half the width and 2 mm of it.

A route that bends rounds the room's corners, whose arcs KentOS and GEOS draw each their way: its points are not
compared and its length is within a centimetre. A blocked point at a passage's mouth is within a millimetre (the mouth
is where two rounded corners meet: x = face − √(R² − (w/2)²)).

    uv run --no-project --with shapely python3 scripts/fixtures/arch_walk_cases.py           # writes the cases
    uv run --no-project --with shapely python3 scripts/fixtures/arch_walk_cases.py --check   # compares with the file
"""

import argparse
import copy
import json
import crmath as math  # correctly rounded: the same digits on every system (docs/adr/9303)
from pathlib import Path

from shapely.geometry import LineString, Point, Polygon
from shapely.ops import unary_union
from shapely.prepared import prep

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.walk.json"
COMMAND = "cad.arch.walk"
MARGIN = 0.001
EDGE_LIMIT = 256
QUAD = 64
CATEGORIES = ("Mobilya", "Mutfak", "Islak hacim")
LEAF = 0.04
NOTHING = {"revision": "same"}
TABLE = "0192f5a1-8a1c-7000-8000-0000000000b1"
LAMP = "0192f5a1-8a1c-7000-8000-0000000000b2"


def UID(n):
    return f"0192f5a1-8a1c-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# ── The drawing's objects ────────────────────────────────────────────────────────────────────────────────────────


def space(id, ring, level="zemin", void=False):
    s = {"kind": "space", "id": id, "layerId": "mahal", "attrs": {}, "parts": [{"outer": {"pts": [P(*p) for p in ring]}}]}
    if level:
        s["arch"] = {"level": level}
    if void:
        s["void"] = True
    return s


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def column(id, x, y, w, d, level="zemin"):
    return {"kind": "column", "id": id, "layerId": "yapi", "attrs": {}, "at": P(x, y), "width": w, "depth": d,
            "arch": {"level": level}}


def wall(id, a, b, width=0.2, level="zemin"):
    return {"kind": "wall", "id": id, "layerId": "yapi", "attrs": {}, "pts": [P(*a), P(*b)], "width": width,
            "justify": "center", "arch": {"level": level}}


def opening(id, w, at, width, form):
    """An opening on wall `w`, `at` along it; its place the start jamb's two face points, then the end jamb's."""
    a, b = w["pts"]
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    l = math.hypot(dx, dy)
    ux, uy = dx / l, dy / l
    nx, ny = -uy, ux
    half = w["width"] / 2.0
    place = [P(a["x"] + ux * s + nx * d, a["y"] + uy * s + ny * d)
             for s in (at - width / 2.0, at + width / 2.0) for d in (half, -half)]
    return {"kind": "opening", "id": id, "layerId": "kapi", "attrs": {}, "host": UID(w["id"]), "at": at,
            "width": width, "form": form, "place": place}


def insert(id, block, x, y, level="zemin"):
    return {"kind": "insert", "id": id, "layerId": "mobilya", "attrs": {}, "block": block, "p": P(x, y), "scale": 1.0,
            "rotation": 0.0, "arch": {"level": level}}


BLOCKS = [
    # A 2 × 1 m table (Mobilya) and a 2 × 1 m lighting block of another category: the same box.
    {"id": TABLE, "name": "Masa", "base": P(0, 0), "card": {"category": "Mobilya/Masalar", "version": 1},
     "entities": [{"kind": "polyline", "id": 1, "layerId": "", "attrs": {}, "pts": [P(*p) for p in rect(0, 0, 2, 1)],
                   "closed": True}]},
    {"id": LAMP, "name": "Aydınlatma", "base": P(0, 0), "card": {"category": "Elektrik", "version": 1},
     "entities": [{"kind": "polyline", "id": 1, "layerId": "", "attrs": {}, "pts": [P(*p) for p in rect(0, 0, 2, 1)],
                   "closed": True}]},
]


def drawing(ents):
    """A drawing of the objects, numbered in their order (a v2 file's ids are its order) and the openings' hosts with
    them."""
    ents = copy.deepcopy(ents)
    renumbered = {}
    for n, e in enumerate(ents, start=1):
        renumbered[UID(e["id"])] = UID(n)
        e["id"] = n
    for e in ents:
        if "host" in e:
            e["host"] = renumbered[e["host"]]
    return {
        "format": "kentos.document", "version": 2, "name": "Dolaşım güzergâhı",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100.0, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                         {"id": "kat1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("mahal", "Mahal"), layer("yapi", "Yapı"), layer("kapi", "Kapı"), layer("mobilya", "Mobilya")],
        "activeLayer": "mahal",
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
        "entities": ents,
    }


# ── The rules, worked out on a drawing ───────────────────────────────────────────────────────────────────────────


def own_level(e):
    return (e.get("arch") or {}).get("level")


def ring_of(e):
    return [(p["x"], p["y"]) for p in e["parts"][0]["outer"]["pts"]]


def clear_width(o):
    """A door's light (fit and frame none) less what its leaf open at 90° takes; an opening's width."""
    if o["form"] == "door":
        return o["width"] - (o["width"] * math.cos(math.pi / 2) + LEAF * math.sin(math.pi / 2))
    return o["width"]


def outline(e, blocks):
    if e["kind"] == "column":
        x, y, w, d = e["at"]["x"], e["at"]["y"], e["width"] / 2.0, e["depth"] / 2.0
        return Polygon(rect(x - w, y - d, x + w, y + d))
    b = next(b for b in blocks if b["id"] == e["block"])
    xs = [p["x"] for q in b["entities"] for p in q["pts"]]
    ys = [p["y"] for q in b["entities"] for p in q["pts"]]
    x, y = e["p"]["x"], e["p"]["y"]
    return Polygon(rect(x + min(xs), y + min(ys), x + max(xs), y + max(ys)))


def edges_of(polys):
    return sum(len(p.exterior.coords) - 1 + sum(len(r.coords) - 1 for r in p.interiors) for p in polys)


def pieces(g):
    return [] if g.is_empty else list(getattr(g, "geoms", [g]))


def shortest(piece, s, e):
    """The shortest polyline in `piece` from s to e over its corners (visibility, Dijkstra); its length."""
    nodes = [s, e] + [c for r in [piece.exterior, *piece.interiors] for c in r.coords[:-1]]
    inside = prep(piece.buffer(1e-9))
    best = [math.inf] * len(nodes)
    done = [False] * len(nodes)
    best[0] = 0.0
    while True:
        at = min((i for i in range(len(nodes)) if not done[i] and best[i] < math.inf), key=lambda i: best[i],
                 default=None)
        if at is None:
            return None
        if at == 1:
            return best[1]
        done[at] = True
        for nxt in range(len(nodes)):
            if done[nxt]:
                continue
            d = best[at] + math.dist(nodes[at], nodes[nxt])
            if d < best[nxt] and inside.covers(LineString([nodes[at], nodes[nxt]])):
                best[nxt] = d


def walk(doc, inp):
    """The route: (output, within), or a refusal."""
    s, e, width = (inp["start"]["x"], inp["start"]["y"]), (inp["end"]["x"], inp["end"]["y"]), inp["width"]
    level = inp.get("level")
    ents, blocks = doc["entities"], doc["blocks"]
    out = {"points": [], "length": 0.0, "width": width}
    floor = [Polygon(ring_of(x)) for x in ents
             if x["kind"] == "space" and own_level(x) == level and not x.get("void") and not x.get("broken")]
    for at in (s, e):
        if not any(f.contains(Point(at)) for f in floor):
            out["blocked"] = P(*at)
            return out, None
    if edges_of(floor) > EDGE_LIMIT:
        return "budget", None
    walls = {x["id"]: x for x in ents if x["kind"] == "wall"}
    portals = []
    for o in ents:
        if o["kind"] != "opening" or o["form"] not in ("door", "opening"):
            continue
        host = next(w for w in walls.values() if UID(w["id"]) == o["host"])
        if own_level(host) != level:
            continue
        p = o["place"]
        a = ((p[0]["x"] + p[1]["x"]) / 2, (p[0]["y"] + p[1]["y"]) / 2)
        b = ((p[2]["x"] + p[3]["x"]) / 2, (p[2]["y"] + p[3]["y"]) / 2)
        half = math.dist((p[0]["x"], p[0]["y"]), (p[1]["x"], p[1]["y"])) / 2 + 0.015
        l = math.dist(a, b)
        ux, uy = (b[0] - a[0]) / l, (b[1] - a[1]) / l
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)

        def at(along, across):
            return (mid[0] + ux * along - uy * across, mid[1] + uy * along + ux * across)
        if not all(any(f.contains(Point(at(0, side))) for f in floor) for side in (-half, half)):
            continue
        w = clear_width(o) / 2
        portals.append(Polygon([at(-w, -half), at(w, -half), at(w, half), at(-w, half)]))
    obstacles = []
    for x in ents:
        if own_level(x) != level:
            continue
        if x["kind"] == "column":
            obstacles.append((x, outline(x, blocks)))
        elif x["kind"] == "insert" and not inp.get("shellOnly"):
            card = next(b for b in blocks if b["id"] == x["block"]).get("card") or {}
            if (card.get("category") or "").split("/")[0].strip() in CATEGORIES:
                obstacles.append((x, outline(x, blocks)))
    cutters = [o for _, o in obstacles]
    if edges_of(floor + portals) + edges_of(cutters) > EDGE_LIMIT:
        return "budget", None
    plan = unary_union(floor + portals).difference(unary_union(cutters))
    r = width / 2.0
    clear = pieces(plan.buffer(-(r + MARGIN), quad_segs=QUAD))
    both = next((c for c in clear if c.contains(Point(s)) and c.contains(Point(e))), None)
    if both is not None:
        line = LineString([s, e])
        if both.buffer(1e-9).covers(line):
            out["points"] = [P(*s), P(*e)]
            out["length"] = math.dist(s, e)
            return out, None
        out["points"] = "$any"
        out["length"] = shortest(both, s, e)
        return out, 0.01
    within = None
    at = s
    piece = next((c for c in clear if c.contains(Point(s))), None)
    if piece is not None:
        # The stretch of the line inside the piece that starts at the start: where it ends, the line leaves.
        stretch = next(g for g in pieces(piece.intersection(LineString([s, e])))
                       if min(math.dist(g.coords[0], s), math.dist(g.coords[-1], s)) < 1e-9)
        at = max((stretch.coords[0], stretch.coords[-1]), key=lambda q: math.dist(q, s))
        within = 0.001
    out["blocked"] = P(*at)
    for x, o in obstacles:
        if o.contains(Point(at)) or o.exterior.distance(Point(at)) <= r + 2 * MARGIN:
            out["obstacle"] = f"$uidOf:{x['id']}"
            break
    return out, within


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────

ROOM = space(1, rect(0, 0, 10, 6))
COLUMN = column(2, 5.0, 3.0, 1.0, 1.0)
CORRIDOR = space(3, rect(0, 0, 10, 1.2))
FULL = column(4, 5.0, 0.6, 1.0, 1.2)
HALL = space(5, rect(0, 0, 10, 2.0))
A = space(6, rect(0, 0, 4.0, 4.0))
B = space(7, rect(4.2, 0, 8.0, 4.0))
WALL = wall(8, (4.1, 0.0), (4.1, 4.0))
UPPER = space(9, rect(0, 0, 10, 6), level="kat1")
L_SHAPE = space(10, [(0, 0), (6, 0), (6, 1.5), (1.5, 1.5), (1.5, 6), (0, 6)])
STRAIGHT = {"start": P(1, 3), "end": P(9, 3), "width": 0.9, "level": "zemin"}
BUDGET = "Bu katta rota hesabı için çok fazla sınır var. Daha küçük bir çizim kapsamı kullanın."

cases = []


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def add(name, ents, inp, op="execute", non_finite=None, refusal=None):
    doc = drawing(ents)
    step = {"op": op, "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    if refusal is not None:
        step["result"] = refusal
    else:
        got, within = walk(doc, inp)
        if got == "budget":
            step["result"] = failed("invalid_input", BUDGET)
        else:
            step["result"] = {"status": "completed", "output": got, "warnings": []}
            if within is not None:
                step["within"] = within
    step["expect"] = NOTHING
    cases.append({"name": name, "setup": doc, "steps": [step]})


add("Açık odada düz yol: iki uç, uzunluğu aralarındaki uzaklık", [ROOM], STRAIGHT)
add("Plan aynı yolu verir, hiçbir şey yazmaz", [ROOM], STRAIGHT, op="plan")
add("Odanın ortasındaki kolonun çevresinden dolanır", [ROOM, COLUMN], STRAIGHT)
add("Yoldaki masa (Mobilya) engeldir: çevresinden dolanılır", [ROOM, insert(11, TABLE, 4.0, 2.5)], STRAIGHT)
add("Yalnız yapı kabuğu: masa yok sayılır, yol düzdür", [ROOM, insert(11, TABLE, 4.0, 2.5)],
    {**STRAIGHT, "shellOnly": True})
add("Başka kategorideki blok (Elektrik) engel değildir", [ROOM, insert(12, LAMP, 4.0, 2.5)], STRAIGHT)
add("Başlangıç mahallerin dışında: rota orada tıkanır", [ROOM], {**STRAIGHT, "start": P(-1, 3)})
add("Bitiş mahallerin dışında: rota bitişte tıkanır", [ROOM], {**STRAIGHT, "end": P(11, 3)})
add("Duvara yarım genişlikten yakın başlangıç: başlangıçta tıkanır", [ROOM], {**STRAIGHT, "start": P(0.3, 3)})
add("Koridoru boydan boya kapatan kolon: önünde tıkanır, engel o kolon", [CORRIDOR, FULL],
    {"start": P(1, 0.6), "end": P(9, 0.6), "width": 0.8, "level": "zemin"})
add("Kolonun yanındaki 80 cm'lik boşluktan 50 cm genişlikle geçilir", [HALL, FULL],
    {"start": P(1, 0.6), "end": P(9, 0.6), "width": 0.5, "level": "zemin"})
add("Aynı boşluk 90 cm genişliğe yetmez: kolonun önünde tıkanır", [HALL, FULL],
    {"start": P(1, 0.6), "end": P(9, 0.6), "width": 0.9, "level": "zemin"})
add("Arada geçiş olmayan iki oda: ilk odanın yürüme sınırında tıkanır", [A, B],
    {"start": P(1, 2), "end": P(7, 2), "width": 0.9, "level": "zemin"})
add("120 cm'lik açıklıktan düz geçilir", [A, B, WALL, opening(13, WALL, 2.0, 1.2, "opening")],
    {"start": P(1, 2), "end": P(7, 2), "width": 0.9, "level": "zemin"})
add("80 cm'lik açıklık 90 cm genişliği geçirmez: ağzının önünde tıkanır",
    [A, B, WALL, opening(13, WALL, 2.0, 0.8, "opening")], {"start": P(1, 2), "end": P(7, 2), "width": 0.9, "level": "zemin"})
add("100 cm'lik kapının 96 cm temiz genişliğinden 90 cm ile geçilir", [A, B, WALL, opening(14, WALL, 2.0, 1.0, "door")],
    {"start": P(1, 2), "end": P(7, 2), "width": 0.9, "level": "zemin"})
add("90 cm'lik kapının 86 cm temiz genişliği 90 cm'ye yetmez", [A, B, WALL, opening(14, WALL, 2.0, 0.9, "door")],
    {"start": P(1, 2), "end": P(7, 2), "width": 0.9, "level": "zemin"})
add("L biçimli koridorun iç köşesi dönülür", [L_SHAPE], {"start": P(5, 0.75), "end": P(0.75, 5), "width": 0.9, "level": "zemin"})
add("Kat süzgeci: 1. katın mahali, zemin kattaki kolon yolda değil", [ROOM, COLUMN, UPPER], {**STRAIGHT, "level": "kat1"})
add("Kat verilmezse katı olan mahaller alınmaz: başlangıçta tıkanır", [ROOM],
    {k: v for k, v in STRAIGHT.items() if k != "level"})
add("10 m genişlik kabul edilir; 6 m'lik odada yürüme alanı kalmaz", [ROOM], {**STRAIGHT, "width": 10.0})
add("256'dan çok kenarlı mahal: rota hesabı yapılmaz",
    [space(15, [(5 + 4 * math.cos(2 * math.pi * k / 300), 3 + 2.8 * math.sin(2 * math.pi * k / 300))
                for k in range(300)])], {**STRAIGHT, "start": P(2, 3), "end": P(8, 3)})
add("Çok dar genişlik", [ROOM], {**STRAIGHT, "width": 0.04},
    refusal=failed("invalid_input", "Geçiş genişliği 0,05–10 m arasında olmalı.", "width"))
add("Çok geniş genişlik", [ROOM], {**STRAIGHT, "width": 10.5},
    refusal=failed("invalid_input", "Geçiş genişliği 0,05–10 m arasında olmalı.", "width"))
add("Olmayan kat", [ROOM], {**STRAIGHT, "level": "yok"},
    refusal=failed("level_not_found", "Çizimde bulunan bir kat seçin.", "level"))
add("Başlangıç sayı değil (NaN)", [ROOM], STRAIGHT, non_finite={"start.x": "NaN"},
    refusal=failed("not_finite", "Çizimde sonlu bir konum seçin.", "start"))
add("Bitiş sonsuz", [ROOM], STRAIGHT, non_finite={"end.y": "Infinity"},
    refusal=failed("not_finite", "Çizimde sonlu bir konum seçin.", "end"))



def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Dolaşım güzergâhı: aynı kattaki en kısa yürüme yolu, tıkandığı yer ve engeli",
            "note": __doc__.split("\n\n", 1)[1].split("\n\n    uv run")[0].strip(),
            "setup": drawing([ROOM]), "cases": cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="the file on disk is what this writes")
    args = ap.parse_args()
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            raise SystemExit(1)
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(cases)} durum)")


if __name__ == "__main__":
    main()
