"""Neighbours' vertices inside an edge an edit changed (docs/adr/3400; plan/map-cad.md MAP-07): shared cases of
cad.entities.stretch (Esnet) and cad.entities.vertex (Yaya dönüştür, Düz kenar yap) with Topolojik düzenleme on.

    python3 scripts/fixtures/topology_inner_cases.py           # writes the files
    python3 scripts/fixtures/topology_inner_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.stretch.inner.json and cad.entities.vertex.inner.json beside
reshape_command_cases.py's and vertex_command_cases.py's files. The rule, computed here without KentOS code:

- An edge of the edited object whose ends moved or whose bulge changed carries the shown neighbours' vertices strictly
  inside it as it was (within 1 µm of it, farther than 1 µm from both its ends): a third object's corner (a T
  junction), the end of a line joining it, a neighbour's own vertex along it. Each goes to the same fraction of the
  edge as it is now (ArcGIS Pro's “stretch topology proportionately”: the edges' relative lengths kept): on a straight
  edge the fraction of its length, on an arc the fraction of its angle.
- Where the edge's bulge changed, a neighbour's edge along it from such a vertex (or from an end of the edge) takes the
  bulge of the same part of the new edge: tan(θ·(t₂ − t₁)/4), θ the new edge's angle (0 for a straight one).
- A locked neighbour stays and is counted (`neighbour_locked`); a hidden one stays. Topolojik düzenleme off: only the
  edited object changes.
- ADR 4200, Köşe sil: a path that loses its vertex V between P and Q, both edges straight, is joined by the straight
  edge P–Q; the old edges P–V and V–Q are the parts P–X and X–Q of it, X at P–V's share of the two lengths, so a
  vertex inside them goes to the same share of the joined edge (the relative lengths kept). An open path's end vertex
  joins nothing: its edge goes and what lay on it stays.

The arcs are computed here from the bulge: the chord's middle pushed off by bulge·chord/2 is the arc's middle; the
centre is on the chord's normal; a place at fraction t is the start turned about the centre by t·θ. The coordinates
compare within 1e-6 m (`within`; a place near 4 420 000 m computed two ways).
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
X, Y = 487000, 4420000
WITHIN = 1e-6
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
S = 4 * math.atan(0.25)  # the angle of D's arc (bulge 0.25)


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


def num(v):
    return int(v) if float(v).is_integer() else v


def P(x, y):
    return {"x": num(X + x), "y": num(Y + y)}


def poly(i, pts, layer_id="parsel", bulges=None):
    e = {"kind": "polygon", "id": i, "layerId": layer_id, "attrs": {}, "pts": [P(*p) for p in pts]}
    if bulges is not None:
        e["bulges"] = bulges
    return e


def line(i, a, b, layer_id="yol"):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(*a), "b": P(*b)}


def arc_of(a, b, bulge):
    """The arc from a to b of `bulge`: its centre, radius and signed angle θ = 4·atan(bulge)."""
    theta = 4 * math.atan(bulge)
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    dx, dy = b[0] - a[0], b[1] - a[1]
    c = math.hypot(dx, dy)
    # The arc's middle: the chord's middle pushed bulge·c/2 to the right of a → b (a positive bulge's side).
    sag = bulge * c / 2
    nx, ny = dy / c, -dx / c
    top = (mx + nx * sag, my + ny * sag)
    # The radius from the chord and the sagitta; the centre r back from the arc's middle towards the chord.
    r = (c * c / 4 + sag * sag) / (2 * abs(sag))
    s = 1 if sag > 0 else -1
    return (top[0] - nx * s * r, top[1] - ny * s * r), r, theta


def turned(c, p, angle):
    dx, dy = p[0] - c[0], p[1] - c[1]
    ca, sa = math.cos(angle), math.sin(angle)
    return (c[0] + dx * ca - dy * sa, c[1] + dx * sa + dy * ca)


# A (1) with B1 (2) and B2 (3) meeting on its north edge at (20, 30), a road (4) joining its south edge at (10, 0),
# C (5) west of it with a vertex of its own on A's west edge at (0, 12), a locked line (6) joining the south edge at
# (30, 0), a hidden one (7) at (25, 0); D (8) with an arc as its south edge (bulge 0.25: its middle (80, −5)), the
# line E (9) joining the arc's middle, F (10) along the arc's western half. J (11) joins A's north edge at (30, 30)
# inside B2; the boundary stone G (12) stands on A's east edge; K (13) ends a millimetre short of the south edge, M (14)
# on its line beyond A. The road R (15) with a branch T2 (16) joining its first edge; the line N (17) with N2 (18)
# joining it; H (19) with a hole whose south edge the island I (20) touches; the stone G2 (21) on D's arc a quarter in.
def on_arc(a, b, bulge, t):
    c, _, theta = arc_of(a, b, bulge)
    return turned(c, a, theta * t)


ENTITIES = [
    poly(1, [(0, 0), (40, 0), (40, 30), (0, 30)]),
    poly(2, [(0, 30), (20, 30), (20, 50), (0, 50)]),
    poly(3, [(20, 30), (40, 30), (40, 50), (20, 50)]),
    line(4, (10, -20), (10, 0)),
    poly(5, [(-20, 0), (0, 0), (0, 12), (0, 30), (-20, 30)]),
    line(6, (30, -15), (30, 0), "kilitli"),
    line(7, (25, -10), (25, 0), "gizli"),
    poly(8, [(60, 0), (100, 0), (100, 30), (60, 30)], bulges=[0.25, 0, 0, 0]),
    line(9, (80, -25), (80, -5)),
    poly(10, [(60, 0), (80, -5), (80, -20), (60, -20)], bulges=[math.tan(S / 8), 0, 0, 0]),
    line(11, (30, 40), (30, 30)),
    {"kind": "point", "id": 12, "layerId": "parsel", "attrs": {}, "p": P(40, 15)},
    line(13, (15, -10), (15, -0.001)),
    line(14, (50, -10), (50, 0)),
    {"kind": "polyline", "id": 15, "layerId": "yol", "attrs": {}, "pts": [P(0, -40), P(40, -40), P(40, -60)]},
    line(16, (20, -60), (20, -40)),
    line(17, (0, -80), (40, -80)),
    line(18, (10, -90), (10, -80)),
    {**poly(19, [(120, 0), (160, 0), (160, 40), (120, 40)]),
     "holes": [{"pts": [P(130, 10), P(150, 10), P(150, 30), P(130, 30)]}]},
    poly(20, [(140, 10), (145, 20), (135, 20)]),
    {"kind": "point", "id": 21, "layerId": "parsel", "attrs": {}, "p": None},
]
ENTITIES[-1]["p"] = P(*on_arc((60, 0), (100, 0), 0.25, 0.25))
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
LOCKED, HIDDEN = {"kilitli"}, {"gizli"}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kenarın ortasındaki köşeler",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 500},
    "origin": {"x": X, "y": Y},
    "layers": [layer("parsel", "Parsel"), layer("yol", "Yol"), layer("kilitli", "Kilitli", locked=True),
               layer("gizli", "Gizli", visible=False)],
    "activeLayer": "parsel", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
UID = lambda i: f"$uidOf:{i}"  # noqa: E731
NEIGHBOUR_LOCKED = "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı."


# ── The geometry, by hand ──

def xy(p):
    return (p["x"] - X, p["y"] - Y)


def paths(e):
    """An object's paths [(pts, bulges, closed)]: a line's two ends, a polyline, an area's ring then its holes."""
    if e["kind"] == "line":
        return [([xy(e["a"]), xy(e["b"])], [0], False)]
    if e["kind"] == "point":
        return []
    pts = [xy(q) for q in e["pts"]]
    closed = e["kind"] == "polygon"
    n = len(pts) if closed else len(pts) - 1
    out = [(pts, (list(e.get("bulges") or []) + [0] * n)[:n], closed)]
    for h in e.get("holes", []):
        hp = [xy(q) for q in h["pts"]]
        out.append((hp, (list(h.get("bulges") or []) + [0] * len(hp))[:len(hp)], True))
    return out


def vertices(e, points):
    if e["kind"] == "point":
        return [xy(e["p"])] if points else []
    return [v for pts, _, _ in paths(e) for v in pts]


def edges(path):
    pts, bulges, closed = path
    n = len(pts)
    return [(pts[k], pts[(k + 1) % n], bulges[k]) for k in range(n if closed else n - 1)]


def at(edge, t):
    a, b, bulge = edge
    if t == 0:
        return a
    if t == 1:
        return b
    if bulge == 0:
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    c, _, theta = arc_of(a, b, bulge)
    return turned(c, a, theta * t)


def fraction(edge, v):
    """Where v lies on the edge, as a fraction; None when not strictly inside it."""
    a, b, bulge = edge
    if bulge == 0:
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        along = ((v[0] - a[0]) * dx + (v[1] - a[1]) * dy) / length
        off = abs((v[0] - a[0]) * dy - (v[1] - a[1]) * dx) / length
        return along / length if off <= 1e-6 and 1e-6 < along < length - 1e-6 else None
    c, r, theta = arc_of(a, b, bulge)
    if abs(math.hypot(v[0] - c[0], v[1] - c[1]) - r) > 1e-6:
        return None
    a0 = math.atan2(a[1] - c[1], a[0] - c[0])
    d = (math.atan2(v[1] - c[1], v[0] - c[0]) - a0) * (1 if theta > 0 else -1) % (2 * math.pi)
    return d / abs(theta) if d < abs(theta) and d * r > 1e-6 and (abs(theta) - d) * r > 1e-6 else None


def part_bulge(edge, t1, t2):
    _, _, bulge = edge
    return 0 if bulge == 0 else math.tan(4 * math.atan(bulge) * (t2 - t1) / 4)


def near(p, q):
    return math.hypot(p[0] - q[0], p[1] - q[1]) <= 1e-6


def rebuilt(e, new_paths=None, new_p=None):
    """`e` with its paths (or a point's place) replaced; all-zero bulges dropped, as the drawing keeps them."""
    out = dict(e)
    if e["kind"] == "point":
        out["p"] = P(*new_p)
        return out
    if e["kind"] == "line":
        (a, b), bulges, _ = new_paths[0]
        if any(x != 0 for x in bulges):
            raise AssertionError("a line made an arc")
        out["a"], out["b"] = P(*a), P(*b)
        return out
    (pts, bulges, closed), *holes = new_paths
    out["pts"] = [P(*q) for q in pts]
    out.pop("bulges", None)
    if any(x != 0 for x in bulges):
        out["bulges"] = bulges + ([] if closed else [0])
    if holes:
        out["holes"] = []
        for hp, hb, _ in holes:
            h = {"pts": [P(*q) for q in hp]}
            if any(x != 0 for x in hb):
                h["bulges"] = hb
            out["holes"].append(h)
    return out


def neighbours_of(edited_ids, points):
    return [e for e in ENTITIES if e["id"] not in edited_ids and e["layerId"] not in HIDDEN
            and (e["kind"] != "point" or points)]


def carry(targets, points, edited_too, pairs=None):
    """The rule over `targets` [(entity, paths before, paths after)]: (the moves, own first then carried, in order;
    the bulge changes; the carried moves alone). `pairs` (Köşe sil, ADR 4200): the edges as they were and as they are,
    path by path, given instead of side by side (a path lost a vertex: no vertex of it moved)."""
    ids = {e["id"] for e, _, _ in targets}
    own, bends, carried = [], [], []
    for e, before, after in targets:
        if pairs is not None:
            continue
        for (bp, bb, _), (ap, ab, _) in zip(before, after):
            own.extend((u, v) for u, v in zip(bp, ap) if u != v)
    for e, before, after in targets:
        cands = neighbours_of(ids, points)
        others = [o for o, _, _ in targets if o["id"] != e["id"]]
        cands_all = cands + (others if edited_too else [])
        held = set()
        seen = []
        for k_pair, (path_b, path_a) in enumerate(zip(before, after)):
            side = pairs[k_pair] if pairs is not None else list(zip(edges(path_b), edges(path_a)))
            for old, new in side:
                if old == new:
                    continue
                places = []
                for o in cands_all:
                    for v in vertices(o, points):
                        t = fraction(old, v)
                        if t is not None and not any(near(v, q) for q, _ in places):
                            places.append((v, t))
                if not edited_too:
                    held = {v for o in others for v in vertices(o, points) if fraction(old, v) is not None}
                for v, t in places:
                    if any(near(v, q) for q in seen) or any(near(v, h) for h in held):
                        continue
                    seen.append(v)
                    to = at(new, t)
                    if to != v:
                        carried.append((v, to))
                if old[2] != new[2]:
                    # The whole edge shared: a neighbour's edge with the same ends (either way) takes the new bulge.
                    bends.append(("whole", old, new))

                    def on(v):
                        if near(v, old[0]):
                            return 0, False
                        if near(v, old[1]):
                            return 1, False
                        return next(((t, True) for q, t in places if near(q, v)), None)
                    for o in cands_all:
                        for k_path, path in enumerate(paths(o)):
                            for k, (u, w, be) in enumerate(edges(path)):
                                tu, tw = on(u), on(w)
                                if tu is None or tw is None or not (tu[1] or tw[1]) or tu[0] == tw[0]:
                                    continue
                                if abs(be - part_bulge(old, tu[0], tw[0])) > 1e-9:
                                    continue
                                bends.append(("part", o["id"], k_path, k, part_bulge(new, tu[0], tw[0])))
    return own + carried, bends, carried


def moved_paths(e, moves, bends, points):
    """`e`'s paths (or place) with the first move at each vertex, the bulge changes on its edges; None if nothing."""
    first = lambda v: next((to for at_, to in moves if near(v, at_)), None)  # noqa: E731
    if e["kind"] == "point":
        to = first(xy(e["p"])) if points else None
        return None if to is None else ("p", to)
    changed = False
    out = []
    for k_path, (pts, bulges, closed) in enumerate(paths(e)):
        new_pts = []
        for v in pts:
            to = first(v)
            changed |= to is not None
            new_pts.append(to if to is not None else v)
        new_b = list(bulges)
        for b in bends:
            if b[0] == "part" and b[1] == e["id"] and b[2] == k_path:
                new_b[b[3]] = b[4]
                changed = True
            elif b[0] == "whole":
                _, old, new = b
                for k, (u, w, be) in enumerate(edges((pts, bulges, closed))):
                    if near(u, old[0]) and near(w, old[1]) and abs(be - old[2]) <= 1e-9:
                        new_b[k] = new[2]
                        changed = True
                    elif near(u, old[1]) and near(w, old[0]) and abs(be + old[2]) <= 1e-9:
                        new_b[k] = -new[2]
                        changed = True
        out.append((new_pts, new_b, closed))
    return ("paths", out) if changed else None


def apply_to(e, result):
    kind, value = result
    return rebuilt(e, new_p=value) if kind == "p" else rebuilt(e, new_paths=value)


def follow(targets, points, edited_too, pairs=None):
    """Topolojik düzenleme over `targets` [(entity, paths before, paths after)]: (the targets as written, the
    neighbours put right in the drawing's order, how many locked)."""
    moves, bends, carried = carry(targets, points, edited_too, pairs)
    written = []
    for e, before, after in targets:
        # A stretched object's vertex at a carried place goes there too (Esnet puts its own stretched objects right).
        if edited_too:
            first = lambda v: next((to for at_, to in carried if near(v, at_)), v)  # noqa: E731
            after = [([first(v) for v in pts], b, c) for pts, b, c in after]
        written.append((e, before, after))
    ids = {e["id"] for e, _, _ in targets}
    own = []
    for e, before, after in written:
        if pairs is not None:
            break
        for (bp, _, _), (ap, _, _) in zip(before, after):
            own.extend((u, v) for u, v in zip(bp, ap) if u != v)
    moves = own + carried
    out, locked = [], 0
    for e in neighbours_of(ids, points):
        result = moved_paths(e, moves, bends, points)
        if result is None:
            continue
        if e["layerId"] in LOCKED:
            locked += 1
            continue
        out.append(apply_to(e, result))
    return [(e, rebuilt(e, new_paths=after)) for e, _, after in written], out, locked


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


SHARED_LOCKED = ("Ortak sınır kilitli katmandaki {} komşu nesneyle paylaşılıyor; hiçbir şey yazılmadı. Kilidi açın ya da "
                 "Topolojik düzenleme › Kilitli komşuda durdur seçeneğini kapatın.")
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def topology_input(points, stop):
    t = {}
    if points:
        t["points"] = True
    if stop:
        t["stopOnLocked"] = True
    return t


# ── Esnet ──

def inside(p, w):
    (x0, y0), (x1, y1) = w
    return min(x0, x1) <= p[0] <= max(x0, x1) and min(y0, y1) <= p[1] <= max(y0, y1)


def stretch_case(name, uids, w, dx, dy, topology=True, points=False, stop=False):
    if uids is None:
        targets_e = [e for e in ENTITIES if e["layerId"] not in HIDDEN | LOCKED
                     and any(inside(v, w) for v in vertices(e, True))]
    else:
        targets_e = [BY_ID[i] for i in uids]
    targets = []
    for e in targets_e:
        before = paths(e) if e["kind"] != "point" else []
        after = [([(v[0] + dx, v[1] + dy) if inside(v, w) else v for v in pts], b, c) for pts, b, c in before]
        targets.append((e, before, after))
    inp = {"window": {"min": P(*w[0]), "max": P(*w[1])}, "dx": num(dx), "dy": num(dy)}
    if uids is not None:
        inp["uids"] = [UID(i) for i in uids]
    written = [(e, rebuilt(e, new_paths=after)) for e, _, after in targets]
    follow_, locked = [], 0
    if topology:
        inp["topology"] = topology_input(points, stop)
        written, follow_, locked = follow(targets, points, True)
        if locked and stop:
            return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
                "code": "shared_boundary_locked", "message": SHARED_LOCKED.format(locked), "path": "topology"}},
                "expect": UNCHANGED}]}
    warnings = [{"code": "neighbour_locked", "message": NEIGHBOUR_LOCKED.format(locked)}] if locked else []
    ents = {str(n["id"]): n for _, n in written}
    ents.update({str(n["id"]): n for n in follow_})
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"stretched": [UID(e["id"]) for e, _ in written], "followed": [UID(n["id"]) for n in follow_],
                              "revision": "$current"}, warnings),
         "within": WITHIN,
         "expect": {"ids": IDS, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False,
                    "dirty": True}},
        {"op": "undo", "returns": "Esnet", "expect": {"ids": IDS, "entities": {k: BY_ID[int(k)] for k in ents}}},
    ]}


def stretch_cases():
    ne, se, nw, sw = ((39, 29), (41, 31)), ((39, -1), (41, 1)), ((-1, 29), (1, 31)), ((-1, -1), (1, 1))
    return [
        stretch_case("Kenarın ucu döner: kenarın ortasındaki üçlü birleşim ve çizgi ucu kenarın aynı kesrine gider",
                     [1], ne, 4, 2),
        stretch_case("Güney kenarı döner: T ile bağlanan yolun ucu kenarla gider; kilitli çizgi uyarıyla, gizli, 1 mm "
                     "uzaktaki ve uzantıdaki çizgiler yerinde kalır", [1], se, 0, -4),
        stretch_case("Kenarın iki ucu birden: kenar ötelenir, ortasındaki köşeler de; batı kenarı uzar, komşunun kendi "
                     "ara köşesi oranla kayar", [1], ((-1, 29), (41, 31)), 0, 2),
        stretch_case("Köşe batıya: iki kenar birden döner; yol ucu ve komşunun ara köşesi kenarlarıyla", [1], sw, -2, 0),
        stretch_case("Yaylı kenarın ucu: yay korunur; yayın ortasındaki çizgi ucu ve komşunun köşesi yeni yayın ortasına, "
                     "komşunun yarım yayı aynı bulge ile yeni yayın üstünde", [8], ((99, -1), (101, 1)), 8, 0),
        stretch_case("Topolojik düzenleme kapalı: yalnız nesne esner, kenarın ortasındaki köşeler yerinde", [1], ne, 4, 2,
                     topology=False),
        stretch_case("Kenar kısalır: köşe içeri; üçlü birleşim ve çizgi ucu oranla kenarda kalır", [1], ne, -3, -2),
        stretch_case("Kuzeybatı köşesi yukarı: kuzey kenarı öbür ucu çevresinde döner, batı kenarı uzar", [1], nw, 0, 5),
        stretch_case("Kenar kendi doğrusunda uzar: yol ucu kenarın aynı kesrine kayar (oran korunur)", [1], se, 8, 0),
        stretch_case("Kilitli komşuda durdur: kenarın ortasındaki köşesi kilitli çizginin; hiçbir şey yazılmaz", [1], se,
                     0, -4, stop=True),
        stretch_case("Noktalar da: doğu kenarındaki sınır taşı da kenarla gider", [1], ne, 4, 2, points=True),
        stretch_case("Yolun köşesi: ilk kenarına bağlanan kolun ucu kenarla gider; ikinci kenarda köşe yok", [15],
                     ((39, -41), (41, -39)), 0, 4),
        stretch_case("Çizginin ucu: ortasına bağlanan çizginin ucu onunla döner", [17], ((39, -81), (41, -79)), 0, 8),
        stretch_case("Deliğin köşesi: delik kenarına değen adanın köşesi kenarla gider", [19], ((149, 9), (151, 11)), 0, -2),
        stretch_case("Yayın başı batıya: yaydaki köşeler yeni yayın ortasına; komşunun ortak köşesi de", [8],
                     ((59, -1), (61, 1)), -4, 0),
        stretch_case("Yayın iki ucu birden: yay ötelenir, ortasındaki köşeler de", [8], ((59, -1), (101, 1)), 0, -3),
        stretch_case("Kenarında köşe olmayan iki kenar: yalnız nesne esner", [8], ((99, 29), (101, 31)), 2, 2),
        stretch_case("Noktalar da: yaydaki sınır taşı yayın aynı kesrine", [8], ((99, -1), (101, 1)), 8, 0, points=True),
        stretch_case("Seçimsiz pencere B2'nin köşesini de alır: ikisi de esner, ortak T köşesi ikisinde birlikte kayar "
                     "(sınır ayrılmaz)", None, ne, 4, 2),
        stretch_case("Batı kenarının iki ucu: kenar ötelenir, komşunun ara köşesi de; B1 ve C'nin köşeleri onunla", [1],
                     ((-1, -1), (1, 31)), -3, 0),
    ]


# ── Köşe komutu: Yaya dönüştür, Düz kenar yap ──

def three_point_bulge(a, m, b):
    """The bulge of the arc from a through m to b: its circle through the three, the signed angle from a to b passing
    m (counter-clockwise positive)."""
    (ax, ay), (bx, by), (cx, cy) = a, m, b
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
    ang = lambda q: math.atan2(q[1] - uy, q[0] - ux)  # noqa: E731
    ccw = (ang(b) - ang(a)) % (2 * math.pi)
    through = (ang(m) - ang(a)) % (2 * math.pi)
    sweep = ccw if through < ccw else ccw - 2 * math.pi
    return math.tan(sweep / 4)


def radius_bulge(a, b, r):
    """The shorter arc of radius |r| on the chord, bowing right of travel for a positive r."""
    c = math.hypot(b[0] - a[0], b[1] - a[1])
    theta = 2 * math.asin(c / (2 * abs(r)))
    return math.copysign(math.tan(theta / 4), r)


def vertex_case(name, target_id, action, edge, bulge_after=None, through=None, radius=None, topology=True,
                points=False, stop=False, returns=None):
    target = BY_ID[target_id]
    before = paths(target)
    pts, bulges, closed = before[0]
    a, b = pts[edge], pts[(edge + 1) % len(pts)]
    if through is not None:
        bulge_after = three_point_bulge(a, through, b)
    if radius is not None:
        bulge_after = radius_bulge(a, b, radius)
    inp = {"uid": UID(target_id), "action": action, "place": {"kind": "mid", "part": 0, "ring": 0, "index": edge}}
    if through is not None:
        inp["through"] = P(*through)
    if radius is not None:
        inp["radius"] = radius
    if action == "insert":
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        after = [(pts[:edge + 1] + [mid] + pts[edge + 1:], bulges[:edge] + [0, 0] + bulges[edge + 1:], closed)] + before[1:]
    else:
        new_b = list(bulges)
        new_b[edge] = bulge_after
        after = [(pts, new_b, closed)] + before[1:]
    written = [(target, rebuilt(target, new_paths=after))]
    follow_, locked = [], 0
    if topology:
        inp["topology"] = topology_input(points, stop)
        if action != "insert":
            written, follow_, locked = follow([(target, before, after)], points, False)
        if locked and stop:
            return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
                "code": "shared_boundary_locked", "message": SHARED_LOCKED.format(locked), "path": "topology"}},
                "expect": UNCHANGED}]}
    warnings = [{"code": "neighbour_locked", "message": NEIGHBOUR_LOCKED.format(locked)}] if locked else []
    ents = {str(n["id"]): n for _, n in written}
    ents.update({str(n["id"]): n for n in follow_})
    changed = [target_id] + [n["id"] for n in follow_]
    undo = {"op": "undo", "expect": {"ids": IDS, "entities": {k: BY_ID[int(k)] for k in ents}}}
    if returns:
        undo["returns"] = returns
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": [UID(i) for i in changed], "ids": changed, "revision": "$current"}, warnings),
         "within": WITHIN,
         "expect": {"ids": IDS, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False,
                    "dirty": True}},
        undo,
    ]}


def joined(path, j):
    """Köşe sil (ADR 4200): the path without its vertex j, and its edges as they were against as they are: the two
    straight edges about V as the parts of the straight edge joining their other ends, V's image at the first's share
    of the two lengths; the other edges side by side. An open path's end: its edge goes, the rest side by side."""
    pts, bulges, closed = path
    n = len(pts)
    new_pts = pts[:j] + pts[j + 1:]
    if closed:
        new_b = bulges[:j] + bulges[j + 1:]
        new_b[(j - 1) % (n - 1)] = 0
    else:
        new_b = (bulges[:j] + bulges[j + 1:]) if 0 < j < n - 1 else (bulges[1:] if j == 0 else bulges[:-1])
    after = (new_pts, new_b, closed)
    old_edges = edges(path)
    if not closed and (j == 0 or j == n - 1):
        kept = old_edges[1:] if j == 0 else old_edges[:-1]
        return after, list(zip(kept, edges(after)))
    prev, nxt = (j - 1) % n, (j + 1) % n
    p, v, q = pts[prev], pts[j], pts[nxt]
    l1, l2 = math.hypot(v[0] - p[0], v[1] - p[1]), math.hypot(q[0] - v[0], q[1] - v[1])
    t = l1 / (l1 + l2)
    x = (p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t)
    out = []
    for k, e in enumerate(old_edges):
        if k == prev:
            out.append((e, (p, x, 0)))
        elif k == j:
            out.append((e, (x, q, 0)))
    return after, out


def delete_case(name, target_id, ring, index, topology=True, points=False, stop=False):
    """Köşe sil on `target_id`'s ring (0: its outline, 1: its hole) at `index`, as vertex_case does the others."""
    target = BY_ID[target_id]
    before = paths(target)
    new_path, pair = joined(before[ring], index)
    after = [new_path if k == ring else p for k, p in enumerate(before)]
    pairs = [pair if k == ring else list(zip(edges(p), edges(p))) for k, p in enumerate(before)]
    inp = {"uid": UID(target_id), "action": "remove", "place": {"kind": "vertex", "part": 0, "ring": ring, "index": index}}
    written = [(target, rebuilt(target, new_paths=after))]
    follow_, locked = [], 0
    if topology:
        inp["topology"] = topology_input(points, stop)
        written, follow_, locked = follow([(target, before, after)], points, False, pairs)
        if locked and stop:
            return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
                "code": "shared_boundary_locked", "message": SHARED_LOCKED.format(locked), "path": "topology"}},
                "expect": UNCHANGED}]}
    warnings = [{"code": "neighbour_locked", "message": NEIGHBOUR_LOCKED.format(locked)}] if locked else []
    ents = {str(n["id"]): n for _, n in written}
    ents.update({str(n["id"]): n for n in follow_})
    changed = [target_id] + [n["id"] for n in follow_]
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": [UID(i) for i in changed], "ids": changed, "revision": "$current"}, warnings),
         "within": WITHIN,
         "expect": {"ids": IDS, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False,
                    "dirty": True}},
        {"op": "undo", "returns": "Köşe sil", "expect": {"ids": IDS, "entities": {k: BY_ID[int(k)] for k in ents}}},
    ]}


def vertex_cases():
    # A runs counter-clockwise: Yaya dönüştür bows its edges outwards, a positive bulge of 0.5 (as D's straightened arc
    # comes back to 0). Its edges: 0 south, 1 east, 2 north, 3 west.
    return [
        vertex_case("Yaya dönüştür: kuzey kenarın ortasındaki üçlü birleşim ve çizgi ucu yaya; B1 ve B2'nin kenar "
                    "parçaları yayın parçaları olur", 1, "arc", 2, 0.5),
        vertex_case("Düz kenar yap: yaylı kenarın ortasındaki çizgi ucu ve komşunun köşesi kirişe; komşunun yarım yayı "
                    "düz olur", 8, "straighten", 0, 0),
        vertex_case("Yaya dönüştür, güney kenarı: yol ucu yaya; kilitli çizgi uyarıyla, gizli, 1 mm uzaktaki ve "
                    "uzantıdaki yerinde", 1, "arc", 0, 0.5),
        vertex_case("Yaya dönüştür, batı kenarı: komşunun ara köşesi yaya, iki kenar parçası yayın parçaları", 1, "arc", 3,
                    0.5),
        vertex_case("Yaya dönüştür, doğu kenarı: Noktalar da kapalıyken sınır taşı yerinde", 1, "arc", 1, 0.5),
        vertex_case("Yaya dönüştür, doğu kenarı, Noktalar da: sınır taşı yayın ortasına", 1, "arc", 1, 0.5, points=True),
        vertex_case("Yaya dönüştür noktadan (kuzey kenarı, ortasından uzak bir nokta): köşeler yayın aynı kesrine", 1,
                    "arc", 2, through=(30, 34)),
        vertex_case("Yaya dönüştür yarıçapla (29 m): üçlü birleşim ve çizgi ucu yayın aynı kesrine", 1, "arc", 2,
                    radius=29),
        vertex_case("Yaya dönüştür, Topolojik düzenleme kapalı: yalnız kenar", 1, "arc", 2, 0.5, topology=False),
        vertex_case("Yaya dönüştür, kilitli komşuda durdur: güney kenarında kilitli çizginin ucu; hiçbir şey yazılmaz", 1,
                    "arc", 0, 0.5, stop=True),
        vertex_case("Düz kenar yap, Topolojik düzenleme kapalı: yalnız kenar", 8, "straighten", 0, 0, topology=False),
        vertex_case("Yaya dönüştür, D'nin doğu kenarı: ortasında köşe yok, yalnız kenar", 8, "arc", 1, 0.5),
        vertex_case("Yaya dönüştür, yolun ilk kenarı (açık çoklu çizgi): kolun ucu yayın ortasına", 15, "arc", 0, 0.5),
        vertex_case("Yaya dönüştür, yolun ikinci kenarı: ortasında köşe yok", 15, "arc", 1, 0.5),
        vertex_case("Ortasına köşe ekle: köşe sayısı değişir, kenarın ortasındaki köşeler taşınmaz", 1, "insert", 2,
                    returns="Köşe ekle"),
        vertex_case("Düz kenar yap, Noktalar da: yaydaki sınır taşı kirişin aynı kesrine", 8, "straighten", 0, 0,
                    points=True),
        vertex_case("Yaya dönüştür, H'nin güney kenarı: ortasında köşe yok", 19, "arc", 0, 0.5),
        vertex_case("Yaya dönüştür yarıçapla (20,5 m, yarım çembere yakın): köşeler yayın aynı kesrine", 1, "arc", 2,
                    radius=20.5),
        vertex_case("Yaya dönüştür içeri bakan noktadan (20, 25): yay alanın içine döner, köşeler de", 1, "arc", 2,
                    through=(20, 25)),
        vertex_case("Yaya dönüştür, batı kenarı noktadan (−3, 15): komşunun ara köşesi ve kenar parçaları", 1, "arc", 3,
                    through=(-3, 15)),
        # ADR 4200: Köşe sil joins the two edges; what lay inside them goes to the same share of the joined edge.
        delete_case("Köşe sil, yolun ortası: kolun T ucu birleşen kenarın aynı payına (40 + 20 m'nin 20'si)", 15, 0, 1),
        delete_case("Köşe sil, A'nın güneydoğusu: yol ucu birleşen kenara; kilitli çizgi uyarıyla, gizli, 1 mm uzaktaki "
                    "ve uzantıdaki yerinde", 1, 0, 1),
        delete_case("Köşe sil, Noktalar da: doğu kenarındaki sınır taşı da birleşen kenara", 1, 0, 1, points=True),
        delete_case("Köşe sil, kilitli komşuda durdur: hiçbir şey yazılmaz", 1, 0, 1, stop=True),
        delete_case("Köşe sil, Topolojik düzenleme kapalı: yalnız köşe gider, kol yerinde", 15, 0, 1, topology=False),
        delete_case("Köşe sil, yolun ilk köşesi: kenarı gider, kolun ucu yerinde kalır", 15, 0, 0),
    ]


FILES = [
    ("cad.entities.stretch", "cad.entities.stretch.inner", "Nesneleri esnet: kenarın ortasındaki komşu köşeler",
     stretch_cases),
    ("cad.entities.vertex", "cad.entities.vertex.inner", "Köşe ve kenar: kenarın ortasındaki komşu köşeler",
     vertex_cases),
]


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "Written by scripts/fixtures/topology_inner_cases.py from docs/adr/3400's rule; do not edit by hand.",
            "setup": SETUP, "cases": cases()}


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for command, name, title, cases in FILES:
        text = json.dumps(build(command, title, cases), ensure_ascii=False, indent=2) + "\n"
        path = OUT / f"{name}.json"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)",
                      file=sys.stderr)
                bad = True
            else:
                print(f"{name} durumları güncel.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
