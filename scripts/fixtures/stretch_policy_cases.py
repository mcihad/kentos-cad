"""What Esnet moves and what follows it: shared cases of cad.entities.stretch (docs/adr/0297).

    python3 scripts/fixtures/stretch_policy_cases.py           # writes the file
    python3 scripts/fixtures/stretch_policy_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.stretch.policy.json beside reshape_command_cases.py's cad.entities.stretch.json.
The ADR's rules, computed here without KentOS code:

- Only an object's vertices in the window move. An arc's vertices are its two ends: a window over its middle (or its
  centre) alone leaves it; both ends move it whole; one end moves that end and keeps the arc's chord height (the
  distance from the chord's middle to the arc's middle) on the same side of the chord, as AutoCAD's STRETCH does. A
  polyline's arc edge keeps its bulge. A circle, an ellipse and a text move whole when the window meets them (a
  crossing window: a window inside a circle that does not reach its outline takes nothing, as AutoCAD's) and their
  centre or insertion point is in it.
- What follows other objects does not stretch by itself: an associative hatch (its closed object there), a dimension's
  points tied to objects; they stay with their objects, which the step's end follows. A dimension whose points in the
  window are all tied is left out (listed: `follows_object`); its free points in the window move, its tied ones stay.
- The plan gives the objects that follow as they will be (`dependents`), in the drawing's order; the answer their ids.
  A tied point goes where its place went: a line's start or end, a polygon's vertex. An associative hatch's region is
  its closed object's ring, counter-clockwise from its lowest vertex (the leftmost of the lowest).

Arcs need angles, so their plans compare within 1e-9 (`within`); everything else is exact.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-9
TAU = 2 * math.pi
U = lambda i: f"$uidOf:{i}"  # noqa: E731


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def num(v):
    return int(v) if float(v).is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


def xy(q):
    return (q["x"], q["y"])


def base(i, layer="cizim"):
    return {"id": i, "layerId": layer, "attrs": {}}


def line(i, a, b, layer="cizim"):
    return {"kind": "line", **base(i, layer), "a": p(*a), "b": p(*b)}


def arc(i, c, r, a0, a1):
    return {"kind": "arc", **base(i), "c": p(*c), "r": r, "a0": a0, "a1": a1}


def dimension(i, a, b):
    return {"kind": "dimension", **base(i, "olcu"), "a": p(*a), "b": p(*b), "offset": 1.5, "height": 0.25}


START, END = {"kind": "start"}, {"kind": "end"}


def vertex(i):
    return {"kind": "vertex", "part": 0, "ring": 0, "index": i}


ENTITIES = [
    line(1, (0, 0), (10, 0)),
    arc(2, (30, 0), 5, 0, math.pi),
    arc(3, (50, 0), 5, 0, math.pi / 2),
    {"kind": "polygon", **base(4), "pts": [p(60, 0), p(70, 0), p(70, 10), p(60, 10)]},
    {"kind": "polyline", **base(5), "pts": [p(0, 20), p(10, 20), p(10, 30)], "bulges": [0.5, 0]},
    {"kind": "circle", **base(6), "c": p(80, 5), "r": 3},
    dimension(7, (0, 0), (10, 0)),
    dimension(8, (0, 40), (10, 40)),
    line(9, (20, 40), (30, 40)),
    dimension(10, (20, 40), (30, 40)),
    {"kind": "ellipse", **base(11), "c": p(100, 5), "major": p(4, 0), "ratio": 0.5, "t0": 0, "t1": TAU},
    {"kind": "text", **base(12), "p": p(90, 20), "text": "Oda", "height": 0.5, "rotation": 0},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
HATCH = max(IDS) + 1
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}

# The dimensions' references, bound by a case's first steps (a file cannot name the objects' ids before they have them).
REFS = {
    7: {"a": {"uid": U(1), "place": START}, "b": {"uid": U(1), "place": END}},
    10: {"a": {"uid": U(9), "place": START}},
}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Esnet ve izleyenler",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(50, 20),
        "layers": [node("cizim", "Çizim"), node("olcu", "Ölçü"), node("tarama", "Tarama")],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


def tied(d, ent=None):
    return {**(ent or BY_ID[d]), "refs": copy.deepcopy(REFS[d])}


def tie_steps(dims):
    """cad.dimensions.reassociate binds each dimension's tied points to their places before the case's command."""
    return [{"op": "execute", "command": "cad.dimensions.reassociate",
             "input": {"uid": U(d), "points": [{"point": n, "to": r} for n, r in REFS[d].items()]},
             "result": "$any", "expect": {"entities": {str(d): tied(d)}}} for d in dims]


def hatch_step(associative=True):
    region = {"kind": "point", "at": p(65, 5)}
    if not associative:
        region["associative"] = False
    return {"op": "execute", "command": "cad.entities.hatch",
            "input": {"region": region, "pattern": {"name": "Dolu"}, "layerId": "tarama"},
            "result": "$any", "expect": {"ids": IDS + [HATCH]}}


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

def inside(q, w):
    (x0, y0), (x1, y1) = w
    return min(x0, x1) <= q[0] <= max(x0, x1) and min(y0, y1) <= q[1] <= max(y0, y1)


def mover(w, dx, dy):
    return lambda q: (q[0] + dx, q[1] + dy) if inside(q, w) else q


def norm(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def circle_through(s, m, f):
    ax, ay = s
    bx, by = m
    cx, cy = f
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
    return (ux, uy), math.hypot(ax - ux, ay - uy)


def arc_through(s, m, f):
    """The arc from s through m to f, as counter-clockwise angles."""
    c, r = circle_through(s, m, f)
    a_s, a_m, a_f = (math.atan2(q[1] - c[1], q[0] - c[0]) for q in (s, m, f))
    if norm(a_m - a_s) < norm(a_f - a_s):
        return c, r, norm(a_s), norm(a_f)
    return c, r, norm(a_f), norm(a_s)


def arc_points(e):
    (cx, cy), r = xy(e["c"]), e["r"]
    a0, a1 = e["a0"], e["a1"]
    sweep = norm(a1 - a0) or TAU
    at = lambda t: (cx + r * math.cos(t), cy + r * math.sin(t))  # noqa: E731
    return at(a0), at(a0 + sweep / 2), at(a1)


def stretched_arc(e, w, dx, dy):
    s, m, f = arc_points(e)
    mv = mover(w, dx, dy)
    if not inside(s, w) and not inside(f, w):
        return None
    s2, f2 = mv(s), mv(f)
    if inside(s, w) and inside(f, w):
        m2 = (m[0] + dx, m[1] + dy)
    else:
        # The chord height and its side stay.
        q = ((s[0] + f[0]) / 2, (s[1] + f[1]) / 2)
        h = math.hypot(m[0] - q[0], m[1] - q[1])
        side = 1 if (f[0] - s[0]) * (m[1] - s[1]) - (f[1] - s[1]) * (m[0] - s[0]) >= 0 else -1
        ux, uy = f2[0] - s2[0], f2[1] - s2[1]
        n = math.hypot(ux, uy)
        m2 = ((s2[0] + f2[0]) / 2 - uy / n * h * side, (s2[1] + f2[1]) / 2 + ux / n * h * side)
    c, r, a0, a1 = arc_through(s2, m2, f2)
    return {**e, "c": {"x": c[0], "y": c[1]}, "r": r, "a0": a0, "a1": a1}


def canonical_ccw(pts):
    area = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(pts, pts[1:] + pts[:1]))
    if area < 0:
        pts = [pts[0]] + pts[:0:-1]
    k = min(range(len(pts)), key=lambda i: (pts[i][1], pts[i][0]))
    return pts[k:] + pts[:k]


def hatch(ring, associative=True):
    h = {"kind": "hatch", **base(HATCH, "tarama"), "ring": [p(*q) for q in canonical_ccw(ring)],
         "pattern": {"type": "solid", "angle": 0, "spacing": 1}}
    if associative:
        h["assoc"] = {"outer": U(4), "seed": p(65, 5)}
    return h


PARCEL = [xy(q) for q in BY_ID[4]["pts"]]


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def window(w):
    return {"min": p(*w[0]), "max": p(*w[1])}


def completed(output, warnings=()):
    return {"status": "completed", "warnings": list(warnings), "output": output}


def warning(code, message, path=None):
    w = {"code": code, "message": message}
    if path is not None:
        w["path"] = path
    return w


NOTHING = {"status": "failed", "error": {
    "code": "nothing_to_stretch", "path": "window",
    "message": "Pencerede köşesi olan düzenlenebilir nesne yok. Pencereyi taşınacak köşelerin üstüne verin."}}
FOLLOWS = "Nesne bağlı olduğu nesneyi izler; kendisi esnemedi."
OUTSIDE = "Nesnenin pencerede köşesi yok; esnemedi."


def case(name, w, dx, dy, stretched, dependents=(), uids=None, warnings=(), before=(), within=False, undo=None,
         execute=True):
    """`stretched`: the objects as they will be, in the drawing's order; `dependents`: those that follow."""
    inp = {"window": window(w), "dx": num(dx), "dy": num(dy)}
    if uids is not None:
        inp["uids"] = [U(i) for i in uids]
    plan_out = {"stretched": stretched, "followed": [], "revision": "$current"}
    if dependents:
        plan_out["dependents"] = list(dependents)
    plan = {"op": "plan", "input": inp, "result": completed(plan_out, warnings), "expect": {"revision": "same"}}
    if within:
        plan["within"] = WITHIN
    steps = [*before, plan]
    if execute:
        out = {"stretched": [U(e["id"]) for e in stretched], "followed": [], "revision": "$current"}
        if dependents:
            out["dependents"] = [U(e["id"]) for e in dependents]
        ents = {} if within else {str(e["id"]): e for e in stretched}
        ents.update({str(e["id"]): e for e in dependents})
        steps.append({"op": "execute", "input": inp, "result": completed(out, warnings),
                      "expect": {"entities": ents, "revision": "changed", "canUndo": True}})
        if undo is not None:
            steps.append({"op": "undo", "returns": "Esnet", "expect": {"entities": undo}})
    return {"name": name, "steps": steps}


def refused(name, w, dx, dy, uids=None, warnings=(), before=()):
    inp = {"window": window(w), "dx": num(dx), "dy": num(dy)}
    if uids is not None:
        inp["uids"] = [U(i) for i in uids]
    result = copy.deepcopy(NOTHING)
    expect = {"revision": "same"} if before else UNCHANGED
    return {"name": name, "steps": [*before, {"op": "execute", "input": inp, "result": result, "expect": expect}]}


def moved_line(i, w, dx, dy):
    e = BY_ID[i]
    mv = mover(w, dx, dy)
    return {**e, "a": p(*mv(xy(e["a"]))), "b": p(*mv(xy(e["b"])))}


def cases():
    out = []
    # Yay: köşeleri uçlarıdır.
    out.append(refused("Yalnız ortası pencerede olan yay değişmez: köşeleri uçlarıdır", ((29, 4), (31, 6)), 2, 0))
    out.append(refused("Yalnız merkezi pencerede olan yay değişmez", ((29, -1), (31, 1)), 0, 2))
    w = ((34, -1), (36, 1))
    out.append(case("Bir ucu taşınan yay kiriş yüksekliğini korur (AutoCAD'in STRETCH'i), aynı yanda", w, 2, 1,
                    [stretched_arc(BY_ID[2], w, 2, 1)], within=True))
    w = ((24, -1), (26, 1))
    out.append(case("Öbür ucu taşınan yay da yüksekliğini korur: uç batıya ve aşağı", w, -3, -2,
                    [stretched_arc(BY_ID[2], w, -3, -2)], within=True))
    w = ((24, -1), (36, 6))
    out.append(case("İki ucu pencerede olan yay olduğu gibi taşınır", w, 1, 1, [stretched_arc(BY_ID[2], w, 1, 1)],
                    within=True))
    w = ((49, 4), (51, 6))
    out.append(case("Çeyrek yayın ucu kuzeye: yüksekliği korunur", w, 0, 2, [stretched_arc(BY_ID[3], w, 0, 2)],
                    within=True))
    w = ((29, 4), (31, 6))
    out.append(refused("Seçilen yayın yalnız ortası pencerede: outside_window, esneyen yok", w, 1, 0, uids=[2]))
    # Çoklu çizginin yay kenarı ve öbür türler.
    w = ((9, 19), (11, 21))
    e = BY_ID[5]
    out.append(case("Çoklu çizginin yay kenarı bükümünü korur, köşesi taşınır", w, 1, -1,
                    [{**e, "pts": [p(0, 20), p(11, 19), p(10, 30)]}]))
    out.append(refused("Dairenin yalnız çevresi pencerede: değişmez", ((82, 4), (84, 6)), 1, 0))
    w = ((79, 4), (84, 6))
    out.append(case("Pencere daireye değer ve merkezi içinde: daire taşınır", w, 1, 2, [{**BY_ID[6], "c": p(81, 7)}]))
    out.append(refused("Pencere yalnız dairenin içinde, çevresine değmiyor: daire seçilmez", ((79, 4), (81, 6)), 1, 2))
    w = ((99, 4), (105, 6))
    out.append(case("Pencere elipse değer ve merkezi içinde: elips taşınır", w, -1, 0, [{**BY_ID[11], "c": p(99, 5)}]))
    w = ((89, 19), (91, 21))
    out.append(case("Yazının yeri pencerede: yazı taşınır", w, 0, 1, [{**BY_ID[12], "p": p(90, 21)}]))

    # İlişkili ölçü.
    w = ((9, -1), (11, 1))
    line1 = moved_line(1, w, 2, 0)
    dim7 = tied(7, {**BY_ID[7], "b": p(12, 0)})
    out.append(case("Çizginin ucu esner, bağlı ölçü izler: planda dependents, ölçünün noktası kendiliğinden taşınmaz",
                    w, 2, 0, [line1], [dim7], before=tie_steps([7]),
                    undo={"1": BY_ID[1], "7": tied(7)}))
    out.append(refused("Seçilen ölçünün penceredeki noktası bağlı: follows_object, esneyen yok", w, 2, 0, uids=[7],
                       before=tie_steps([7])))
    w = ((-1, -1), (11, 1))
    out.append(case("Pencere çizgiyi ve ölçüsünü bütünüyle alır: çizgi taşınır, ölçü onu izler", w, 0, 3,
                    [moved_line(1, w, 0, 3)], [tied(7, {**BY_ID[7], "a": p(0, 3), "b": p(10, 3)})], before=tie_steps([7])))
    w = ((9, -1), (11, 1))
    out.append(case("Bağsız ölçünün noktası pencerede: ölçü kendisi esner (bağ kurulmadan)", w, 2, 0,
                    [line1, {**BY_ID[7], "b": p(12, 0)}]))
    w = ((9, 39), (11, 41))
    out.append(case("Serbest ölçünün noktası esner", w, 1, 0, [{**BY_ID[8], "b": p(11, 40)}]))
    w = ((29, 39), (31, 41))
    out.append(case("Ölçünün serbest noktası esner, bağlı noktası nesnesiyle kalır", w, 0, 2,
                    [moved_line(9, w, 0, 2), tied(10, {**BY_ID[10], "b": p(30, 42)})], before=tie_steps([10])))
    w = ((19, 39), (21, 41))
    out.append(case("Ölçünün penceredeki tek noktası bağlı: ölçü esnemez, çizginin başıyla izler", w, 0, 2,
                    [moved_line(9, w, 0, 2)], [tied(10, {**BY_ID[10], "a": p(20, 42)})], before=tie_steps([10])))

    # İlişkili tarama.
    w = ((69, 9), (71, 11))
    poly = {**BY_ID[4], "pts": [p(60, 0), p(70, 0), p(72, 11), p(60, 10)]}
    ring = [(60, 0), (70, 0), (72, 11), (60, 10)]
    out.append(case("Alanın köşesi esner, ilişkili tarama kendisi esnemez, alanı izler: planda yeni bölgesiyle",
                    w, 2, 1, [poly], [hatch(ring)], before=[hatch_step()], undo={"4": BY_ID[4], str(HATCH): hatch(PARCEL)}))
    out.append(refused("Seçilen ilişkili tarama: follows_object, esneyen yok", w, 2, 1, uids=[HATCH], before=[hatch_step()]))
    out.append(case("İlişkisiz tarama kendi köşesiyle esner", w, 2, 1, [poly, hatch(ring, associative=False)],
                    before=[hatch_step(associative=False)]))
    w = ((69, 9), (71, 11))
    out.append(case("Yer değiştirme sıfır: hiçbir şey yazılmaz, izleyen de yok", w, 0, 0, [], before=[hatch_step()],
                    execute=False))

    # Seçimdeki uyarıların birlikte söylenmesi.
    w = ((9, -1), (11, 1))
    out.append(case("Seçimde çizgi ve bağlı ölçüsü: ölçü follows_object uyarısıyla atlanır, çizgiyi izler", w, 2, 0,
                    [line1], [dim7], uids=[1, 7], warnings=[warning("follows_object", FOLLOWS, "uids[1]")],
                    before=tie_steps([7])))
    w = ((34, -1), (36, 1))
    out.append(case("Seçimde iki yay, biri pencerede değil: outside_window", w, 1, 0, [stretched_arc(BY_ID[2], w, 1, 0)],
                    uids=[2, 3], warnings=[warning("outside_window", OUTSIDE, "uids[1]")], within=True))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.stretch", "commandVersion": 1,
            "title": "Esnet: yalnız penceredeki köşeler, yay ve izleyenler",
            "note": "ADR 0297. Yayın köşeleri uçlarıdır, bir ucu taşınan yay kiriş yüksekliğini korur; bağlı olduğu nesneyi "
                    "izleyen (ilişkili tarama, ölçünün bağlı noktası) kendisi esnemez, izler; planın dependents'ı. Bağımsız "
                    "başvuru: scripts/fixtures/stretch_policy_cases.py.",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    path = OUT / "cad.entities.stretch.policy.json"
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{path.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    path.write_text(text)
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
