"""The shared cases of cad.entities.fillet and .chamfer between two objects' straight sides (docs/adr/0295).

    python3 scripts/fixtures/corner_sides_cases.py           # writes the files
    python3 scripts/fixtures/corner_sides_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.{fillet,chamfer}.sides.json (the commands' first files, edge_command_cases.py's,
hold their other cases). The checks, codes, paths and messages are written here by hand from the ADR. What the commands
do is computed from the operations' definitions, without KentOS code:

- A side is a line, or an open polyline's first or last straight edge: the edge nearest the pick. The sides' lines meet
  at the corner X; each keeps the part on its pick's side, its far end the one reaching farthest from X.
- Köşe yuvarla: the tangent points lie r / tan(φ/2) from X along each side, the centre r / sin(φ/2) along the
  bisector; the arc is the minor one, counter-clockwise, its angles in [0, 2π). Pah: the cut points lie d1 and d2 from X.
- A line becomes its kept end and its tangent (or cut) point, in its own direction. A polyline's free end moves to the
  tangent point; when the kept end is its free end the corner would land on its inner vertex, refused.
- Two objects: the arc or the cut is a new object from the first side. One polyline's first and last edges: the path
  closes, its closing edge (last vertex to first) the arc as a bulge tan(sweep / 4), minus when the arc runs clockwise
  from the last vertex, or the cut; with nothing between, the ends meet and the last vertex goes.

Every side is axis-parallel, so X, the tangent and cut points and the arc's angles are exact; sin and tan of the half
angle and the bulge's tan are computed to 60 digits by their series and rounded to the nearest double.
"""
import json
import math
import sys
from decimal import Decimal, getcontext
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
X, Y = 487000, 4420000
getcontext().prec = 60


def P(x, y):
    return {"x": X + x, "y": Y + y}


def W(p):
    """An absolute point as the wire writes it."""
    return {"x": num(p[0]), "y": num(p[1])}


def A(p):
    """A point given from the origin, absolute (the core computes in absolute coordinates; so does this)."""
    return (float(X + p[0]), float(Y + p[1]))


def num(v):
    return int(v) if float(v).is_integer() else v


def layer(i, name, locked=False):
    style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": style, "children": []}


def line(i, a, b, layer_id="yapi", **extra):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, **extra, "a": P(*a), "b": P(*b)}


def path(i, pts, layer_id="yapi", bulges=None, closed=False, **extra):
    e = {"kind": "polyline", "id": i, "layerId": layer_id, "attrs": {}, **extra, "pts": [P(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if closed:
        e["closed"] = True
    return e


ENTITIES = [
    line(30, (0, 0), (10, 0), color="#E5484D", lineWeight=0.7),
    path(31, [(14, 4), (14, 14), (24, 14)], color="#3E63DD", attrs={"Tür": "Bordür"}),
    path(32, [(40, 0), (50, 0), (50, 10)]),
    line(33, (44, 14), (56, 14)),
    # Its first edge an arc bulging south: the arc edge is refused, its last edge is straight.
    path(34, [(60, 0), (70, 0), (70, 8)], bulges=[0.5, 0, 0]),
    path(35, [(74, 12), (84, 12), (84, 20)]),
    # A square open at its south-west corner: its first and last edges close it.
    path(36, [(100, 4), (100, 10), (110, 10), (110, 0), (104, 0)], attrs={"Ada": "102"}),
    {"kind": "polygon", "id": 37, "layerId": "yapi", "attrs": {}, "pts": [P(120, 0), P(130, 0), P(130, 10), P(120, 10)]},
    path(38, [(140, 0), (150, 0), (150, 10), (140, 10)], closed=True),
    line(39, (10, 16), (18, 16)),
    line(40, (0, 30), (10, 30), layer_id="kilitli"),
    path(41, [(14, 34), (14, 40)]),
    # Lines drawn end to end (Tüm köşeler on lines): a 10 × 6 rectangle, a stair with a 2 m step, two lines in line.
    line(42, (200, 0), (210, 0), color="#E5484D"),
    line(43, (210, 0), (210, 6)),
    line(44, (210, 6), (200, 6)),
    line(45, (200, 6), (200, 0)),
    line(46, (220, 0), (230, 0)),
    line(47, (230, 0), (230, 2)),
    line(48, (230, 2), (240, 2)),
    line(49, (240, 2), (240, 12)),
    line(50, (250, 0), (255, 0)),
    line(51, (255, 0), (260, 0)),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 52

SETUP = {
    "format": "kentos.document", "version": 1, "name": "İki nesnenin köşesi",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 500},
    "origin": {"x": X, "y": Y},
    "layers": [layer("yapi", "Yapı"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}

NOT_STRAIGHT = ("İki düz kenar verin: çizgi ya da açık çoklu çizginin ilk ya da son kenarı; çoklu çizginin ve alanın iç "
                "köşesi kendi köşesiyle (vertex) verilir.")
INNER_EDGE = "Çoklu çizgide yalnız ilk ya da son kenar başka bir kenarla köşe yapar; iç köşe için köşenin kendisini (vertex) verin."
ARC_EDGE = "Seçilen kenar yay; yalnız düz kenar köşe yapar."
INNER_END = ("Köşe çoklu çizginin serbest ucunda değil, iç köşesinde kalıyor; kenarın öbür yanına tıklayın ya da köşeyi "
             "kendi köşesiyle (vertex) verin.")
SAME_END = "Aynı uç iki kez seçildi; yolu kapatmak için ilk ve son kenarını seçin."
PARALLEL = "Çizgiler paralel ya da seçilen tarafta çizgi yok."
ALL_KINDS = "Tüm köşeler çoklu çizgide, alanda ve uçları birbirine değen çizgilerde işlenir."
LINES_NO_CORNER = "Çizgilerin buluştuğu yerde köşe yok: ya aynı doğrultudalar ya da bir noktada ikiden çok uç buluşuyor."
FITS_NONE = "Hiçbir köşeye sığmadı; daha küçük bir değer verin ya da kenarları uzatın."
LOCKED = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."


# ── The corner, from its definition ─────────────────────────────────────────────────────────────────────────────────

def _series(x, first, step):
    total, term, n = Decimal(0), first, step
    while abs(term) > Decimal(10) ** -58:
        total += term
        term *= -x * x / ((n + 1) * (n + 2))
        n += 2
    return total


def sin_(x):
    d = Decimal(x)
    return float(_series(d, d, 1))


def tan_(x):
    d = Decimal(x)
    return float(_series(d, d, 1) / _series(d, Decimal(1), 0))


def norm_angle(a):
    r = math.fmod(a, 2 * math.pi)
    return r + 2 * math.pi if r < 0 else r


def pts_of(e):
    return [(float(p["x"]), float(p["y"])) for p in e["pts"]]


def side(eid, pick):
    """The side a pick names: ("line", a, b) or ("path", edge, a, b); the edge nearest the pick (straight edges here,
    and the arc edge's pick is on the arc itself)."""
    e = BY_ID[eid]
    if e["kind"] == "line":
        return ("line", None, (float(e["a"]["x"]), float(e["a"]["y"])), (float(e["b"]["x"]), float(e["b"]["y"])))
    pts = pts_of(e)

    def dist(a, b):
        t = ((pick[0] - a[0]) * (b[0] - a[0]) + (pick[1] - a[1]) * (b[1] - a[1])) / ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
        t = max(0.0, min(1.0, t))
        return math.hypot(pick[0] - a[0] - t * (b[0] - a[0]), pick[1] - a[1] - t * (b[1] - a[1]))
    edge = min(range(len(pts) - 1), key=lambda i: (dist(pts[i], pts[i + 1]), i))
    return ("path", edge, pts[edge], pts[edge + 1])


def corner(s1, p1, s2, p2):
    """X, and each side's direction away from it and its kept (far) end."""
    (_, _, a1, b1), (_, _, a2, b2) = s1, s2
    if a1[1] == b1[1]:
        x = (a2[0], a1[1])
    else:
        x = (a1[0], a2[1])

    def kept(a, b, pick):
        d = ((b[0] - a[0]) / math.hypot(b[0] - a[0], b[1] - a[1]), (b[1] - a[1]) / math.hypot(b[0] - a[0], b[1] - a[1]))
        u = d if (pick[0] - x[0]) * d[0] + (pick[1] - x[1]) * d[1] >= 0 else (-d[0], -d[1])
        reach = lambda p: (p[0] - x[0]) * u[0] + (p[1] - x[1]) * u[1]  # noqa: E731
        far = a if reach(a) >= reach(b) else b
        return u, far
    return x, kept(a1, b1, p1), kept(a2, b2, p2)


def fillet(s1, p1, s2, p2, r):
    x, (u1, far1), (u2, far2) = corner(s1, p1, s2, p2)
    if r == 0:
        return (far1, x), (far2, x), None
    phi = math.acos(max(-1.0, min(1.0, u1[0] * u2[0] + u1[1] * u2[1])))
    dt = r / tan_(phi / 2)
    t1 = (x[0] + u1[0] * dt, x[1] + u1[1] * dt)
    t2 = (x[0] + u2[0] * dt, x[1] + u2[1] * dt)
    h = math.hypot(u1[0] + u2[0], u1[1] + u2[1])
    bis = ((u1[0] + u2[0]) / h, (u1[1] + u2[1]) / h)
    dc = r / sin_(phi / 2)
    c = (x[0] + bis[0] * dc, x[1] + bis[1] * dc)
    a0 = norm_angle(math.atan2(t1[1] - c[1], t1[0] - c[0]))
    a1 = norm_angle(math.atan2(t2[1] - c[1], t2[0] - c[0]))
    if norm_angle(a1 - a0) > math.pi:
        a0, a1 = a1, a0
    return (far1, t1), (far2, t2), ("arc", c, r, a0, a1)


def chamfer(s1, p1, s2, p2, d1, d2):
    x, (u1, far1), (u2, far2) = corner(s1, p1, s2, p2)
    if d1 == 0 and d2 == 0:
        return (far1, x), (far2, x), None
    c1 = (x[0] + u1[0] * d1, x[1] + u1[1] * d1)
    c2 = (x[0] + u2[0] * d2, x[1] + u2[1] * d2)
    return (far1, c1), (far2, c2), ("cut", c1, c2)


def moved(s, kept, n):
    """The polyline vertex the corner moves; None when it would be the inner one."""
    _, i, a, _ = s
    m = i + 1 if kept[0] == a else i
    return m if m in (0, n - 1) else None


def after(eid, s, kept):
    e = BY_ID[eid]
    if s[0] == "line":
        a, b = s[2], s[3]
        k = (kept[1], kept[0]) if kept[0] == b and kept[0] != a else kept
        return {**e, "a": W(k[0]), "b": W(k[1])}
    pts = pts_of(e)
    m = moved(s, kept, len(pts))
    assert m is not None
    pts[m] = kept[1]
    return {**e, "pts": [W(p) for p in pts]}


def closed(eid, one, two, piece):
    e = BY_ID[eid]
    pts = pts_of(e)
    n = len(pts)
    for s, kept in (one, two):
        m = moved(s, kept, n)
        assert m is not None
        pts[m] = kept[1]
    bulges = list(e.get("bulges") or [0] * n)
    out = dict(e)
    if piece is None:
        pts.pop()
        bulges = bulges[: n - 1]
    elif piece[0] == "arc":
        _, c, r, a0, a1 = piece
        start = (c[0] + r * math.cos(a0), c[1] + r * math.sin(a0))
        ccw = math.hypot(start[0] - pts[-1][0], start[1] - pts[-1][1]) <= math.hypot(start[0] - pts[0][0], start[1] - pts[0][1])
        b = tan_(norm_angle(a1 - a0) / 4)
        bulges[n - 1] = b if ccw else -b
    else:
        bulges[n - 1] = 0
    out["pts"] = [W(p) for p in pts]
    if any(b != 0 for b in bulges):
        out["bulges"] = [num(b) for b in bulges]
    else:
        out.pop("bulges", None)
    out["closed"] = True
    return out


def piece_entity(base, piece, eid):
    e = {"kind": "arc" if piece[0] == "arc" else "line", "id": eid, "layerId": base["layerId"], "attrs": {}}
    for k in ("color", "lineWeight"):
        if k in base:
            e[k] = base[k]
    if piece[0] == "arc":
        _, c, r, a0, a1 = piece
        e.update({"c": W(c), "r": num(r), "a0": num(a0), "a1": num(a1)})
    else:
        e.update({"a": W(piece[1]), "b": W(piece[2])})
    return e


def line_corners(ids, kind, size):
    """Tüm köşeler on lines (docs/adr/0295): corners where exactly two ends meet bit for bit and the lines are not in
    line; each end moves back by the corner's size; a line whose two corners need more than it leaves both. Answers
    the lines' new ends, the pieces (with the line they come from), done and skipped, in the command's order."""
    ends = {k: [(float(BY_ID[k]["a"]["x"]), float(BY_ID[k]["a"]["y"])), (float(BY_ID[k]["b"]["x"]), float(BY_ID[k]["b"]["y"]))] for k in ids}
    order = list(ids)
    corners = []
    for ki, k in enumerate(order):
        for e in (0, 1):
            at = [(j, f) for jj, j in enumerate(order) for f in (0, 1) if (j, f) != (k, e) and ends[j][f] == ends[k][e]]
            if len(at) == 1:
                j, f = at[0]
                if (ki, e) < (order.index(j), f) and j != k:
                    corners.append(((k, e), (j, f)))

    def away(k, e):
        p, q = ends[k][e], ends[k][1 - e]
        l = math.hypot(q[0] - p[0], q[1] - p[1])
        return ((q[0] - p[0]) / l, (q[1] - p[1]) / l), l
    planned = []
    for (k, e), (j, f) in corners:
        (u1, l1), (u2, l2) = away(k, e), away(j, f)
        phi = math.acos(max(-1.0, min(1.0, u1[0] * u2[0] + u1[1] * u2[1])))
        if not (1e-6 <= phi <= math.pi - 1e-6):
            planned.append(None)
            continue
        d1, d2 = (size / tan_(phi / 2),) * 2 if kind == "fillet" else size
        planned.append((ends[k][e], u1, u2, phi, d1, d2, l1, l2))
    take = [p is not None and p[4] > 0 and p[5] > 0 and p[4] <= p[6] + 1e-9 and p[5] <= p[7] + 1e-9 for p in planned]
    need, by = {k: [0.0, 0.0] for k in order}, {k: [None, None] for k in order}
    for c, ((k, e), (j, f)) in enumerate(corners):
        if take[c]:
            need[k][e], need[j][f] = planned[c][4], planned[c][5]
            by[k][e], by[j][f] = c, c
    for k in order:
        (a, b) = ends[k]
        if by[k][0] is not None and by[k][1] is not None and need[k][0] + need[k][1] > math.hypot(b[0] - a[0], b[1] - a[1]) + 1e-9:
            take[by[k][0]] = take[by[k][1]] = False
    moved = {k: [None, None] for k in order}
    pieces, done, skipped = [], 0, 0
    for c, ((k, e), (j, f)) in enumerate(corners):
        if planned[c] is None:
            continue
        if not take[c]:
            skipped += 1
            continue
        at, u1, u2, phi, d1, d2, _, _ = planned[c]
        moved[k][e] = (at[0] + u1[0] * d1, at[1] + u1[1] * d1)
        moved[j][f] = (at[0] + u2[0] * d2, at[1] + u2[1] * d2)
        if kind == "fillet":
            t = size / tan_(phi / 2)
            h = math.hypot(u1[0] + u2[0], u1[1] + u2[1])
            bis = ((u1[0] + u2[0]) / h, (u1[1] + u2[1]) / h)
            kk = size / sin_(phi / 2)
            centre = (at[0] + bis[0] * kk, at[1] + bis[1] * kk)
            ang = lambda u: math.atan2(at[1] + u[1] * t - centre[1], at[0] + u[0] * t - centre[0])  # noqa: E731
            a0, a1 = ang(u1), ang(u2)
            ccw = ((math.fmod(a1 - a0, 2 * math.pi)) + 2 * math.pi) % (2 * math.pi) < math.pi
            a0, a1 = (a0, a1) if ccw else (a1, a0)
            pieces.append((k, ("arc", centre, size, norm_angle(a0), norm_angle(a1))))
        else:
            pieces.append((k, ("cut", moved[k][e], moved[j][f])))
        done += 1
    updates = []
    for k in order:
        if moved[k] == [None, None]:
            continue
        a = moved[k][0] or ends[k][0]
        b = moved[k][1] or ends[k][1]
        updates.append((k, {**BY_ID[k], "a": W(a), "b": W(b)}))
    return updates, pieces, done, skipped


def lines_all_case(name, kind, ids, size):
    updates, pieces, done, skipped = line_corners(ids, kind, size)
    field = {"radius": num(size)} if kind == "fillet" else {"distance": num(size[0]), "second": num(size[1])}
    uids = [UID(i) for i in ids]
    ents, created = {}, []
    for n, (k, piece) in enumerate(pieces):
        ents[str(NEXT + n)] = piece_entity(BY_ID[k], piece, NEXT + n)
        created.append(NEXT + n)
    changed = [UID(k) for k, _ in updates]
    for k, e in updates:
        ents[str(k)] = e
    label = "Köşe yuvarla" if kind == "fillet" else "Pah"
    return {"name": name, "steps": [
        {"op": "execute", "input": {"corner": {"kind": "all", "uids": uids}, **field},
         "result": completed({"changed": changed, "created": [UID(c) for c in created], "done": done, "skipped": skipped,
                              "revision": "$current"}),
         "expect": {"ids": IDS + created, "entities": ents, "revision": "changed", "canUndo": True}},
        {"op": "undo", "returns": label, "expect": {"ids": IDS, "entities": {str(k): BY_ID[k] for k in ids}}},
    ]}


def lines_refusals(kind):
    field = {"radius": 1} if kind == "fillet" else {"distance": 1}
    big = {"radius": 9} if kind == "fillet" else {"distance": 9}
    return [
        {"name": "Tüm köşeler: uçları buluşmayan çizgi köşe değildir", "steps": [{"op": "execute",
         "input": {"corner": {"kind": "all", "uids": [UID(42), UID(43), UID(30)]}, **field},
         "result": {"status": "failed", "error": {"code": "not_a_corner", "message": ALL_KINDS, "path": "corner.uids[2]"}}, "expect": UNCHANGED}]},
        {"name": "Tüm köşeler: aynı doğrultudaki çizgilerde köşe yok", "steps": [{"op": "execute",
         "input": {"corner": {"kind": "all", "uids": [UID(50), UID(51)]}, **field},
         "result": {"status": "failed", "error": {"code": "not_a_corner", "message": LINES_NO_CORNER, "path": "corner.uids[0]"}}, "expect": UNCHANGED}]},
        {"name": "Tüm köşeler: hiçbir çizgi köşesine sığmıyor", "steps": [{"op": "execute",
         "input": {"corner": {"kind": "all", "uids": [UID(42), UID(43), UID(44), UID(45)]}, **big},
         "result": {"status": "failed", "error": {"code": f"{kind}_failed", "message": FITS_NONE, "path": "corner.uids"}}, "expect": UNCHANGED}]},
    ]


# ── Cases ────────────────────────────────────────────────────────────────────────────────────────────────────────────

def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def refused(name, kind, input_, code, message, path_):
    field = {"radius": 1} if kind == "fillet" else {"distance": 1}
    step = {"op": "execute", "input": {**input_, **field}, "result": {"status": "failed", "error": {"code": code, "message": message, "path": path_}},
            "expect": UNCHANGED}
    return {"name": name, "steps": [step]}


def picks(first, p1, second, p2):
    return {"kind": "lines", "first": {"uid": UID(first), "at": P(*p1)}, "second": {"uid": UID(second), "at": P(*p2)}}


def size_input(kind, size):
    if kind == "fillet":
        return {"radius": num(size)}
    return {"distance": num(size[0]), "second": num(size[1])}


def run(kind, first, p1, second, p2, size):
    p1, p2 = A(p1), A(p2)
    s1, s2 = side(first, p1), side(second, p2)
    if kind == "fillet":
        return s1, s2, fillet(s1, p1, s2, p2, size)
    return s1, s2, chamfer(s1, p1, s2, p2, *size)


def two_case(name, kind, first, p1, second, p2, size, trim=True, plan=False):
    s1, s2, (k1, k2, piece) = run(kind, first, p1, second, p2, size)
    inp = {"corner": picks(first, p1, second, p2), **size_input(kind, size)}
    if not trim:
        inp["trim"] = False
    made = piece_entity(BY_ID[first], piece, NEXT) if piece else None
    ents = {}
    changed = []
    if trim:
        ents = {str(first): after(first, s1, k1), str(second): after(second, s2, k2)}
        changed = [UID(first), UID(second)]
    if made:
        ents[str(NEXT)] = made
    if plan:
        return {"name": name, "steps": [{"op": "plan", "input": inp,
                "result": completed({"changed": [ents[str(first)], ents[str(second)]], "created": [{**made, "id": 0}] if made else [],
                                     "done": 1, "skipped": 0, "revision": "$current"}),
                "expect": UNCHANGED}]}
    label = "Köşe yuvarla" if kind == "fillet" else "Pah"
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": changed, "created": [UID(NEXT)] if made else [], "done": 1, "skipped": 0, "revision": "$current"}),
         "expect": {"ids": IDS + ([NEXT] if made else []), "entities": ents, "revision": "changed", "canUndo": True}},
        {"op": "undo", "returns": label, "expect": {"ids": IDS, "entities": {str(first): BY_ID[first], str(second): BY_ID[second]}}},
    ]}


def close_case(name, kind, p1, p2, size):
    s1, s2, (k1, k2, piece) = run(kind, 36, p1, 36, p2, size)
    inp = {"corner": picks(36, p1, 36, p2), **size_input(kind, size)}
    label = "Köşe yuvarla" if kind == "fillet" else "Pah"
    return {"name": name, "steps": [
        {"op": "execute", "input": inp,
         "result": completed({"changed": [UID(36)], "created": [], "done": 1, "skipped": 0, "revision": "$current"}),
         "expect": {"ids": IDS, "entities": {"36": closed(36, (s1, k1), (s2, k2), piece)}, "revision": "changed", "canUndo": True}},
        {"op": "undo", "returns": label, "expect": {"ids": IDS, "entities": {"36": BY_ID[36]}}},
    ]}


def refusals(kind):
    too_big = "Yarıçap bu çizgiler için çok büyük." if kind == "fillet" else "Pah mesafesi çizgi boyundan büyük."
    big = {"radius": 40} if kind == "fillet" else {"distance": 40, "second": 40}
    return [
        refused("Çoklu çizginin iç kenarı başka nesneyle köşe yapmaz", kind, {"corner": picks(30, (5, 0), 36, (105, 10))},
                "not_a_corner", INNER_EDGE, "corner"),
        refused("Yay kenarı köşe yapmaz", kind, {"corner": picks(34, (65, -2.5), 35, (80, 12))}, "not_a_corner", ARC_EDGE, "corner"),
        refused("Alanın kenarı başka nesneyle köşe yapmaz; köşesi kendi köşesiyle verilir", kind,
                {"corner": picks(37, (125, 0), 30, (5, 0))}, "not_a_corner", NOT_STRAIGHT, "corner"),
        refused("Kapalı çoklu çizginin kenarı başka nesneyle köşe yapmaz", kind,
                {"corner": picks(38, (145, 0), 30, (5, 0))}, "not_a_corner", NOT_STRAIGHT, "corner"),
        refused("Köşe çoklu çizginin iç köşesine düşüyor", kind, {"corner": picks(31, (14, 8), 39, (11, 16))},
                "not_a_corner", INNER_END, "corner"),
        refused("Aynı yolun aynı ucu iki kez", kind, {"corner": picks(36, (100, 6), 36, (100, 8))}, "not_a_corner", SAME_END, "corner"),
        refused("Paralel kenarlar: çizgi ve çoklu çizginin ilk kenarı", kind, {"corner": picks(30, (5, 0), 35, (80, 12))},
                "not_a_corner", PARALLEL, "corner"),
        refused("Kilitli katmandaki çizgi ve çoklu çizgi", kind, {"corner": picks(41, (14, 36), 40, (5, 30))},
                "layer_locked", LOCKED, "corner.second.uid"),
        {"name": "Ölçü kenarlara sığmıyor", "steps": [{"op": "execute", "input": {"corner": picks(30, (5, 0), 31, (14, 8)), **big},
                                                       "result": {"status": "failed", "error": {"code": f"{kind}_failed", "message": too_big, "path": "corner"}},
                                                       "expect": UNCHANGED}]},
    ]


def fillet_cases():
    return [
        two_case("Çizgi ve çoklu çizginin ilk kenarı 2 m yarıçapla: çizgi uzar, yolun serbest ucu teğet noktasına gelir, yay çizgiden yeni nesne",
                 "fillet", 30, (5, 0), 31, (14, 8), 2.0),
        two_case("Önce çoklu çizgi seçilirse yay onun katmanını, rengini alır", "fillet", 31, (14, 8), 30, (5, 0), 2.0),
        two_case("Sıfır yarıçap çizgiyle yolun ucunu buluştukları noktada birleştirir", "fillet", 30, (5, 0), 31, (14, 8), 0.0),
        two_case("Kırp kapalı: kenarlar kalır, yalnız yay eklenir", "fillet", 30, (5, 0), 31, (14, 8), 2.0, trim=False),
        two_case("Plan: çizgi, yol ve yay yazılacakları gibi; hiçbir şey yazılmaz", "fillet", 30, (5, 0), 31, (14, 8), 2.0, plan=True),
        two_case("Çoklu çizginin son kenarı ve çizgi 3 m: çizginin köşeyi aşan kısmı kesilir", "fillet", 32, (50, 5), 33, (46, 14), 3.0),
        two_case("İki çoklu çizginin uçları: yaylı kenarı olan yolun son kenarı ve öbürünün ilk kenarı", "fillet", 34, (70, 4), 35, (80, 12), 2.0),
        close_case("Aynı yolun ilk ve son kenarı 2 m yarıçapla: yol kapanır, kapanan kenar saat yönünde dönen yay", "fillet", (100, 7), (107, 0), 2.0),
        close_case("Aynı yolun son ve ilk kenarı, ters sırayla: aynı kapanış", "fillet", (107, 0), (100, 7), 2.0),
        close_case("Aynı yolun uçları sıfır yarıçapla: uçlar buluşur, son köşe düşer, yol kapanır", "fillet", (100, 7), (107, 0), 0.0),
        close_case("Aynı yolun uçları 4 m yarıçapla", "fillet", (100, 8), (108, 0), 4.0),
        lines_all_case("Tüm köşeleri yuvarla, uçları buluşan dört çizgi 1 m: dört yay, her çizgi iki ucundan kısalır", "fillet", [42, 43, 44, 45], 1.0),
        lines_all_case("Tüm köşeleri yuvarla, merdiven 1,5 m: kısa basamağın iki köşesi birlikte sığmaz, ikisi de atlanır", "fillet", [46, 47, 48, 49], 1.5),
        lines_all_case("Tüm köşeleri yuvarla, çizgiler ters sırayla verilir: yay önce verilen çizgiden", "fillet", [45, 44, 43, 42], 2.0),
        *lines_refusals("fillet"),
        *refusals("fillet"),
    ]


def chamfer_cases():
    return [
        two_case("Çizgi ve çoklu çizginin ilk kenarına 2 ve 3 m pah: çizgi uzar, yolun ucu pah noktasına gelir, pah çizgiden yeni çizgi",
                 "chamfer", 30, (5, 0), 31, (14, 8), (2.0, 3.0)),
        two_case("Önce çoklu çizgi: birinci mesafe yolda, pah onun renginde", "chamfer", 31, (14, 8), 30, (5, 0), (2.0, 3.0)),
        two_case("Sıfır mesafeler çizgiyle yolun ucunu buluştukları noktada birleştirir", "chamfer", 30, (5, 0), 31, (14, 8), (0.0, 0.0)),
        two_case("Kırp kapalı: kenarlar kalır, yalnız pah çizgisi eklenir", "chamfer", 30, (5, 0), 31, (14, 8), (2.0, 3.0), trim=False),
        two_case("Plan: çizgi, yol ve pah yazılacakları gibi; hiçbir şey yazılmaz", "chamfer", 30, (5, 0), 31, (14, 8), (2.0, 3.0), plan=True),
        two_case("Çoklu çizginin son kenarı ve çizgi: çizginin köşeyi aşan kısmı kesilir", "chamfer", 32, (50, 5), 33, (46, 14), (1.0, 2.0)),
        two_case("İki çoklu çizginin uçları", "chamfer", 34, (70, 4), 35, (80, 12), (2.0, 2.0)),
        close_case("Aynı yolun ilk ve son kenarına 2 ve 3 m pah: yol kapanır, kapanan kenar pah", "chamfer", (100, 7), (107, 0), (2.0, 3.0)),
        close_case("Aynı yolun son ve ilk kenarı, ters sırayla: birinci mesafe son kenarda", "chamfer", (107, 0), (100, 7), (2.0, 3.0)),
        close_case("Aynı yolun uçları sıfır mesafeyle: uçlar buluşur, son köşe düşer", "chamfer", (100, 7), (107, 0), (0.0, 0.0)),
        close_case("Aynı yolun uçlarına eşit 1 m pah", "chamfer", (100, 7), (107, 0), (1.0, 1.0)),
        lines_all_case("Tüm köşelere pah, uçları buluşan dört çizgi 1 ve 2 m: birinci mesafe önce verilen çizgide", "chamfer", [42, 43, 44, 45], (1.0, 2.0)),
        lines_all_case("Tüm köşelere pah, merdiven 1,5 m: kısa basamağın köşeleri atlanır", "chamfer", [46, 47, 48, 49], (1.5, 1.5)),
        lines_all_case("Tüm köşelere pah, çizgiler ters sırayla", "chamfer", [45, 44, 43, 42], (1.0, 2.0)),
        *lines_refusals("chamfer"),
        *refusals("chamfer"),
    ]


FILES = [
    ("cad.entities.fillet", "Köşe yuvarla: iki nesnenin düz kenarları", fillet_cases),
    ("cad.entities.chamfer", "Pah kır: iki nesnenin düz kenarları", chamfer_cases),
]


def build(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "Written by scripts/fixtures/corner_sides_cases.py from the operations' definitions (docs/adr/0250, 0295); do not edit by hand.",
        "setup": SETUP, "cases": cases(),
    }


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for command, title, cases in FILES:
        doc = build(command, title, cases)
        assert len(doc["cases"]) >= 20, (command, len(doc["cases"]))
        assert_no_negative_zero(doc, command)
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path_ = OUT / f"{command}.sides.json"
        if check:
            if not path_.exists() or path_.read_text(encoding="utf-8") != text:
                print(f"{path_.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = True
            else:
                print(f"{command}.sides durumları güncel: {len(doc['cases'])} durum.")
        else:
            path_.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path_.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
