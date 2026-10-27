#!/usr/bin/env python3
"""A wall's temporary distances and Kaydır (docs/adr/0710), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.walls.distances.json and cad.walls.edit.shift.json; `--check` compares and writes
nothing. What does not join after a move is said by scripts/fixtures/wall_join_cases.py's exact solver (through
wall_command_cases.join_warnings).

The rules, as ADR 0710 gives them:
- an edge of a wall's path (a closed one's last runs back to its first point) measured on each side: the nearest
  straight edge of a wall on the same level (its own other edges too) running parallel to it (within 0.001°) and
  overlapping it along its line by more than 1e-9 m; the clear distance from the edge's face on that side (the left
  face `offset + w/2` for a centred wall, `offset` for a left one, `offset + w` for a right one; the right face that
  less the width) to the other's face nearer it, square to the edge; where: the middle of the overlap. Equal
  distances (within 1e-9) go to the wall first in the drawing (the one measured comes first), then its lower edge;
- Kaydır moves the edge's line `by` to its left. A vertex between two edges slides along the other edge (refused when
  it is an arc or runs along the same line, or another wall's end is there). A path end (its join not `none`): one
  other open wall's end there (within 1 mm, its join not `none`) slides it and that end along that wall's last edge
  (refused when the two run in one line, within 1°); two meeting in one line slide it along that line; more walls
  refuse; none: along the edge of a wall whose body it stands in, else square to the edge. Other open walls' ends
  standing in the edge's body (away from its ends) slide along their own last edge. Every edge that changed keeps a
  length above 1 mm and its way, else refused. The walls that changed: the one moved, then those that followed in
  that order (its start's, its end's, then the drawing's order);
- Mesafe: Kaydır by the gap on that side less the distance (left), or the distance less the gap (right);
- openings stay where they are: one on a wall whose path changed has its `at` read back from its jambs (the start
  moving along the wall shifts it), and its jambs written from its wall again.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_command_cases as wc  # noqa: E402  (the warnings of what does not join)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
TIE = 1e-9
JOIN = 1e-3
PARALLEL = 1.745_329_251_994_33e-5
INLINE = math.sin(math.radians(1.0))
STEP = "Duvarı düzenle"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def wall(id, pts, width=0.2, layer="duvar", **extra):
    return {"kind": "wall", "id": id, "layerId": layer, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
            "justify": extra.pop("justify", "center"), **extra}


def uid(i):
    """The setup's persistent ids: one per object, in order."""
    return f"0192f5a1-7100-7000-8000-{i:012x}"


def opening(id, host, at, width, place):
    return {"kind": "opening", "id": id, "layerId": "kapi", "attrs": {}, "host": uid(host), "at": at,
            "width": width, "form": "opening", "place": [P(*p) for p in place]}


SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                                    {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}}
ENTITIES = [
    # Room A: four walls round 0..5 × 0..4, a partition standing in its south and north walls, two openings.
    wall(1, [(0, 0), (5, 0)]),
    wall(2, [(5, 0), (5, 4)]),
    wall(3, [(5, 4), (0, 4)]),
    wall(4, [(0, 4), (0, 0)]),
    wall(5, [(2.5, 0), (2.5, 4)], width=0.1),
    opening(6, 2, 2.0, 0.9, [(4.9, 1.55), (5.1, 1.55), (4.9, 2.45), (5.1, 2.45)]),
    opening(7, 3, 1.5, 0.9, [(3.95, 3.9), (3.95, 4.1), (3.05, 3.9), (3.05, 4.1)]),
    # Room B: a closed rectangular wall.
    wall(8, [(0, 10), (5, 10), (5, 14), (0, 14)], closed=True),
    # A wall alone; a host and a wall standing in it (a tee).
    wall(9, [(0, 20), (0, 23)]),
    wall(10, [(0, 30), (8, 30)]),
    wall(11, [(4, 33), (4, 30)]),
    # Two walls end to end in one line; three walls at one end; two in one line and a third standing on them.
    wall(12, [(0, 40), (3, 40)]),
    wall(13, [(3, 40), (6, 40)]),
    wall(14, [(0, 50), (3, 50)]),
    wall(15, [(3, 50), (3, 53)]),
    wall(16, [(3, 50), (5, 48)]),
    wall(17, [(0, 60), (3, 60)]),
    wall(18, [(3, 60), (6, 60)]),
    wall(19, [(3, 60), (3, 63)]),
    # An arc wall; an L-shaped wall of two edges.
    wall(20, [(0, 70), (4, 70)], bulges=[0.3, 0.0]),
    wall(21, [(0, 80), (4, 80), (4, 83)]),
    # A corner whose other wall is on a locked layer.
    wall(22, [(0, 90), (4, 90)]),
    wall(23, [(4, 90), (4, 93)], layer="kilitli"),
    # Levels: a corner on another level, a parallel wall on another level and one on the same.
    wall(24, [(0, 100), (4, 100)], arch={"level": "zemin"}),
    wall(25, [(4, 100), (4, 103)], arch={"level": "kat-1"}),
    wall(26, [(0, 102), (4, 102)], arch={"level": "kat-1"}),
    wall(27, [(0, 104), (4, 104)], arch={"level": "zemin"}),
    # An end that joins nothing (`none`).
    wall(28, [(0, 110), (4, 110)], joins={"end": "none"}),
    wall(29, [(4, 110), (4, 113)]),
    # A left-justified wall with a wall on each side.
    wall(30, [(0, 120), (4, 120)], justify="left"),
    wall(31, [(0, 121), (4, 121)]),
    wall(32, [(1, 118), (3, 118)]),
    # Two walls at the same distance; one beside and not overlapping; one 0.01° off.
    wall(33, [(0, 130), (4, 130)]),
    wall(34, [(0, 131), (2, 131)]),
    wall(35, [(2, 131), (4, 131)]),
    wall(36, [(0, 140), (2, 140)]),
    wall(37, [(3, 141), (5, 141)]),
    wall(38, [(0, 150), (4, 150)]),
    wall(39, [(0, 151), (4, 151 + 4 * math.tan(math.radians(0.01)))]),
    {"kind": "line", "id": 40, "layerId": "cizim", "attrs": {}, "a": P(0, 160), "b": P(4, 160)},
]
SETUP = {
    "format": "kentos.document", "version": 2, "name": "Duvar kaydırma", "settings": SETTINGS, "origin": P(0, 0),
    "layers": [wc.layer("duvar", "Duvar"), wc.layer("cizim", "Çizim"), wc.layer("kilitli", "Kilitli katman", locked=True),
               wc.layer("kapi", "Kapı")],
    "activeLayer": "duvar", "styles": {"items": [], "categories": []}, "entities": ENTITIES,
    "uids": [uid(e["id"]) for e in ENTITIES],
}
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}


# ── Geometry ─────────────────────────────────────────────────────────────────────────────────────────────────────

def xy(p):
    return (p["x"], p["y"])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def left(u):
    return (-u[1], u[0])


def along(p, d, t):
    return (p[0] + d[0] * t, p[1] + d[1] * t)


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def unit(a, b):
    l = dist(a, b)
    return ((b[0] - a[0]) / l, (b[1] - a[1]) / l)


def left_face(w):
    j = w.get("justify", "center")
    return w.get("offset", 0.0) + {"left": 0.0, "center": w["width"] / 2.0, "right": w["width"]}[j]


def level(w):
    return (w.get("arch") or {}).get("level")


def edges(w):
    return len(w["pts"]) if w.get("closed") else len(w["pts"]) - 1


def bulge(w, i):
    return (w.get("bulges") or [0.0] * len(w["pts"]))[i]


def edge(w, k):
    if k >= edges(w) or bulge(w, k) != 0.0:
        return None
    pts = w["pts"]
    a, b = xy(pts[k]), xy(pts[(k + 1) % len(pts)])
    return (a, b) if dist(a, b) > 0 else None


def joins(w, end):
    return (w.get("joins") or {}).get(["start", "end"][end], "auto")


def frame(w, k):
    n = edges(w)
    if k >= n:
        raise Refused("operation.edge", f"Duvarın {k}. kenarı yok; tek kenarı 0." if n == 1 else
                      f"Duvarın {k}. kenarı yok; kenarları 0 ile {n - 1} arasında.")
    e = edge(w, k)
    if e is None:
        raise Refused("operation.edge", "Duvarın bu kenarı yay; geçici ölçü ve kaydırma düz kenardadır.")
    a, b = e
    u = unit(a, b)
    return a, u, left(u), dist(a, b)


class Refused(Exception):
    def __init__(self, path, message, code="invalid_wall"):
        super().__init__(message)
        self.path, self.message, self.code = path, message, code


def walls_of(state, first):
    """The drawing's walls, `first` first, then the others in the drawing's order."""
    others = [i for i in sorted(state) if state[i]["kind"] == "wall" and i != first]
    return [first] + others


def gaps(state, first, k):
    order = walls_of(state, first)
    me = state[first]
    a, u, n, l = frame(me, k)
    lf = left_face(me)
    rf = lf - me["width"]
    best = [None, None]

    def take(side, g):
        if best[side] is None or g["distance"] < best[side]["distance"] - TIE:
            best[side] = g
    for j in order:
        other = state[j]
        if level(other) != level(me):
            continue
        for e in range(edges(other)):
            if j == first and e == k:
                continue
            ed = edge(other, e)
            if ed is None:
                continue
            c, d = ed
            v = unit(c, d)
            if abs(cross(u, v)) > PARALLEL:
                continue
            tc, td = dot(sub(c, a), u), dot(sub(d, a), u)
            lo, hi = max(0.0, min(tc, td)), min(l, max(tc, td))
            if hi - lo <= TIE:
                continue
            sgn = 1.0 if dot(left(v), n) > 0 else -1.0
            sd = dot(sub(c, a), n)
            olf = left_face(other)
            fl, fr = sd + sgn * olf, sd + sgn * (olf - other["width"])
            near, near_left = (fl, True) if fl <= fr else (fr, False)
            far, far_left = (fl, True) if fl >= fr else (fr, False)
            base = along(a, u, (lo + hi) / 2.0)
            if near >= lf - TIE:
                take(0, {"wall": j, "edge": e, "left": near_left, "distance": near - lf,
                         "from": along(base, n, lf), "to": along(base, n, near)})
            if far <= rf + TIE:
                take(1, {"wall": j, "edge": e, "left": far_left, "distance": rf - far,
                         "from": along(base, n, rf), "to": along(base, n, far)})
    return best


def ends_at(state, order, first, p):
    out = []
    for j in order[1:]:
        o = state[j]
        if level(o) != level(state[first]) or o.get("closed"):
            continue
        n = len(o["pts"])
        for end, vertex, seg in ((0, 0, 0), (1, n - 1, n - 2)):
            if joins(o, end) == "none" or dist(xy(o["pts"][vertex]), p) > JOIN:
                continue
            if bulge(o, seg) != 0.0:
                raise Refused("operation", "Kaydırılan duvara bağlı bir duvarın ucu yay; o duvarı düz kenarla bitirin.")
            inward = (xy(o["pts"][0]), xy(o["pts"][1])) if end == 0 else (xy(o["pts"][n - 1]), xy(o["pts"][n - 2]))
            out.append((j, vertex, unit(*inward)))
    return out


def host_at(state, order, first, p):
    for j in order[1:]:
        o = state[j]
        if level(o) != level(state[first]):
            continue
        lf = left_face(o)
        rf = lf - o["width"]
        for e in range(edges(o)):
            ed = edge(o, e)
            if ed is None:
                continue
            c, d = ed
            v = unit(c, d)
            t, off = dot(sub(p, c), v), dot(sub(p, c), left(v))
            if -JOIN <= t <= dist(c, d) + JOIN and rf - JOIN <= off <= lf + JOIN:
                return v
    return None


def slide(p, d, n, by):
    dn = dot(d, n)
    return along(p, d, by / dn) if abs(dn) > PARALLEL else None


STUCK = "Kaydırılan duvarın ucundaki duvar onunla aynı doğrultuda; uç kaydırılamıyor."


def end_goes(state, order, first, end, p, u, n, by):
    if joins(state[first], end) == "none":
        return along(p, n, by), None
    meeting = ends_at(state, order, first, p)
    if not meeting:
        h = host_at(state, order, first, p)
        if h is None:
            return along(p, n, by), None
        q = slide(p, h, n, by)
        if q is None:
            raise Refused("operation", STUCK)
        return q, None
    if len(meeting) == 1:
        j, vertex, d = meeting[0]
        if abs(cross(u, d)) <= INLINE:
            raise Refused("operation", "Kaydırılan duvar ucunda aynı doğrultudaki bir duvara bağlı (uç uca); önce ikisini ayırın ya da birlikte taşıyın.")
        q = slide(p, d, n, by)
        if q is None:
            raise Refused("operation", STUCK)
        return q, (j, vertex)
    for i, x in enumerate(meeting):
        for y in meeting[i + 1:]:
            if abs(cross(x[2], y[2])) <= INLINE and dot(x[2], y[2]) < 0:
                q = slide(p, x[2], n, by)
                if q is None:
                    raise Refused("operation", STUCK)
                return q, None
    raise Refused("operation", "Kaydırılan duvarın ucunda birden çok duvar birleşiyor; uç kaydırılamıyor.")


def shift(state, first, k, by):
    """The walls that change, as {id: new points}, in order: the one moved first."""
    order = walls_of(state, first)
    me = state[first]
    a, u, n, l = frame(me, k)
    pts = [xy(p) for p in me["pts"]]
    np_ = len(pts)
    count = edges(me)
    moved = {first: list(pts)}

    def put(j, vertex, q):
        if j not in moved:
            moved[j] = [xy(p) for p in state[j]["pts"]]
        moved[j][vertex] = q
    b = pts[(k + 1) % np_]
    for vertex, is_first in ((k, True), ((k + 1) % np_, False)):
        p = pts[vertex]
        if not me.get("closed") and vertex in (0, np_ - 1):
            q, other = end_goes(state, order, first, 0 if vertex == 0 else 1, p, u, n, by)
            put(first, vertex, q)
            if other is not None:
                put(other[0], other[1], q)
            continue
        side = (k + count - 1) % count if is_first else (k + 1) % count
        ed = edge(me, side)
        if ed is None:
            raise Refused("operation", "Kaydırılan kenarın bitişiğindeki kenar yay; yalnız düz kenarlar arasında kaydırılır.")
        v = unit(*ed)
        if abs(cross(u, v)) <= PARALLEL:
            raise Refused("operation", "Kaydırılan kenarın bitişiğindeki kenar aynı doğrultuda; o köşeyi kaldırın.")
        if ends_at(state, order, first, p):
            raise Refused("operation", "Kaydırılan kenarın köşesine başka bir duvarın ucu bağlı; kaydırılamıyor.")
        put(first, vertex, slide(p, v, n, by))
    lf = left_face(me)
    rf = lf - me["width"]
    for j in order[1:]:
        o = state[j]
        on = len(o["pts"])
        if level(o) != level(me) or o.get("closed"):
            continue
        for end, vertex, seg in ((0, 0, 0), (1, on - 1, on - 2)):
            e = xy(o["pts"][vertex])
            if joins(o, end) == "none" or dist(e, a) <= JOIN or dist(e, b) <= JOIN:
                continue
            t, off = dot(sub(e, a), u), dot(sub(e, a), n)
            if t < -JOIN or t > l + JOIN or off < rf - JOIN or off > lf + JOIN:
                continue
            if bulge(o, seg) != 0.0:
                raise Refused("operation", "Kaydırılan duvara saplanan bir duvarın ucu yay; o duvarı düz kenarla bitirin.")
            inward = (xy(o["pts"][0]), xy(o["pts"][1])) if end == 0 else (xy(o["pts"][on - 1]), xy(o["pts"][on - 2]))
            q = slide(e, unit(*inward), n, by)
            if q is not None:
                put(j, vertex, q)
    for j, new in moved.items():
        old = [xy(p) for p in state[j]["pts"]]
        for s in range(edges(state[j])):
            i, i2 = s, (s + 1) % len(old)
            if old[i] == new[i] and old[i2] == new[i2]:
                continue
            if dist(new[i], new[i2]) <= JOIN or dot(sub(new[i2], new[i]), sub(old[i2], old[i])) <= 0:
                raise Refused("operation", f"Bu kaydırma bir duvar kenarını yok ediyor ya da ters çeviriyor ({abs(by):.3f} m); daha küçük bir değer yazın.")
    return moved


# ── Openings: read back from their jambs, jambs from their wall (ADR 0341 §3) ──────────────────────────────────────

def path_s(pts, q):
    """The distance along an open straight path to the point nearest `q`."""
    best, at, run = None, 0.0, 0.0
    for a, b in zip(pts, pts[1:]):
        l = dist(a, b)
        u = unit(a, b)
        t = min(max(dot(sub(q, a), u), 0.0), l)
        p = along(a, u, t)
        d = dist(p, q)
        if best is None or d < best - 1e-12:
            best, at = d, run + t
        run += l
    return at


def point_at(pts, s):
    run = 0.0
    for a, b in zip(pts, pts[1:]):
        l = dist(a, b)
        if s <= run + l + 1e-12:
            return along(a, unit(a, b), s - run), unit(a, b)
        run += l
    a, b = pts[-2], pts[-1]
    return b, unit(a, b)


def jambs(w, at, width):
    pts = [xy(p) for p in w["pts"]]
    lf = left_face(w)
    rf = lf - w["width"]
    out = []
    for s in (at - width / 2.0, at + width / 2.0):
        p, u = point_at(pts, s)
        n = left(u)
        out += [along(p, n, lf), along(p, n, rf)]
    return out


def carried(state, moved):
    """The openings of the walls that changed, `at` read back from their jambs, jambs written again."""
    out = {}
    for i, e in sorted(state.items()):
        if e["kind"] != "opening":
            continue
        host = next(i for i in state if uid(i) == e["host"])
        if host not in moved:
            continue
        place = [xy(p) for p in e["place"]]
        centre = (sum(p[0] for p in place) / 4.0, sum(p[1] for p in place) / 4.0)
        new_pts = moved[host]
        at = path_s(new_pts, centre)
        w = dict(state[host], pts=[P(*p) for p in new_pts])
        o = copy.deepcopy(e)
        o["at"] = at
        o["place"] = [P(*p) for p in jambs(w, at, e["width"])]
        out[i] = o
    return out


# ── The steps ────────────────────────────────────────────────────────────────────────────────────────────────────

def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def before():
    return {"ids": IDS, "revision": "same"}


def num(x):
    return wc.num(x)


def edit(state, inp):
    """(result, changed entities by id) or (refusal, None)."""
    named = wc.named(inp, "Düzenlenecek duvar verilmedi.")
    if isinstance(named, dict):
        return named, None
    for at, i in named:
        if state[i]["kind"] != "wall":
            return refusal("not_a_wall", "Nesne duvar değil; Duvarı düzenle yalnız duvarları değiştirir.", f"uids[{at}]"), None
    op = inp["operation"]
    kind = op["kind"]
    if kind == "shift":
        if not math.isfinite(op["by"]):
            return refusal("not_finite", "Kaydırma sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", "operation.by"), None
    else:
        if not math.isfinite(op["distance"]):
            return refusal("not_finite", "Mesafe sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", "operation.distance"), None
        if op["distance"] < 0:
            return refusal("invalid_wall", f"Mesafe {num(op['distance'])}; sıfır ya da daha büyük olmalı.", "operation.distance"), None
    if len(named) != 1:
        return refusal("invalid_wall", "Kaydır ve Mesafe tek duvarla çalışır; bir duvar verin.", "uids"), None
    first = named[0][1]
    k = op.get("edge", 0)
    try:
        g = gaps(state, first, k)
        if kind == "shift":
            by = op["by"]
        else:
            side = 0 if op["side"] == "left" else 1
            if g[side] is None:
                return refusal("invalid_wall", f"Duvarın {['sol', 'sağ'][side]} yanında ona paralel bir duvar yok; mesafe ölçülemiyor. Kaydır ile uzaklık verin.", "operation.side"), None
            by = g[0]["distance"] - op["distance"] if side == 0 else op["distance"] - g[1]["distance"]
        if abs(by) <= TIE:
            return refusal("nothing_to_set", "Duvar zaten bu konumda; kaydırılacak bir şey yok.", "operation"), None
        moved = shift(state, first, k, by)
    except Refused as r:
        return refusal(r.code, r.message, r.path), None
    blocked = next((state[j] for j in moved if j != first and state[j]["layerId"] == "kilitli"), None)
    if blocked is not None:
        return refusal("layer_locked", "Kaydırma “Kilitli katman” katmanındaki bağlı bir duvarı da değiştirir ve o katman kilitli. Kilidi Katmanlar panelinden açın.", "operation"), None
    changed = {j: dict(state[j], pts=[P(*p) for p in pts]) for j, pts in moved.items()}
    order = list(moved)
    walls_out = [changed[order[0]]]
    joined = [changed[j] for j in order[1:]]
    warn = wc.join_warnings(state, [(j, changed[j]) for j in order])
    output = {"walls": walls_out, "revision": "$current"}
    if joined:
        output["joined"] = joined
    return {"status": "completed", "output": output, "warnings": warn}, {**changed, **carried(state, moved)}


def step(op, inp, non_finite=None, state=None):
    state = state if state is not None else {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    result, changed = edit(state, wc.with_non_finite(inp, non_finite or {}))
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    if changed is None or op == "validate":
        if changed is not None:
            # Doğrula says nothing but whether it would write.
            result = {"status": "completed", "output": None, "warnings": []}
        s["result"] = result
        s["expect"] = before()
        return s, state
    s["result"] = result
    s["within"] = 1e-9
    if op == "plan":
        # What follows the walls in the preview: the openings whose jambs move with them (one whose `at` alone
        # changes stays where it is).
        moved_places = [e for i, e in sorted(changed.items()) if e["kind"] == "opening" and
                        any(abs(a["x"] - b["x"]) > 1e-12 or abs(a["y"] - b["y"]) > 1e-12
                            for a, b in zip(e["place"], state[i]["place"]))]
        if moved_places:
            s["result"] = copy.deepcopy(result)
            s["result"]["output"]["followers"] = {"updated": moved_places}
        s["expect"] = before()
        return s, state
    s["expect"] = {"ids": IDS, "entities": {str(i): e for i, e in sorted(changed.items())}, "canUndo": True,
                   "dirty": True, "revision": "changed"}
    state = {**state, **changed}
    return s, state


def u(i):
    return f"$uidOf:{i}"


def shift_in(i, by, edge=None):
    op = {"kind": "shift", "by": by}
    if edge is not None:
        op["edge"] = edge
    return {"uids": [u(i)], "operation": op}


def distance_in(i, side, d, edge=None):
    op = {"kind": "distance", "side": side, "distance": d}
    if edge is not None:
        op["edge"] = edge
    return {"uids": [u(i)], "operation": op}


def edit_cases():
    c = []

    def add(name, steps):
        assert name not in [x["name"] for x in c], name
        c.append({"name": name, "steps": steps})
    first, after = step("execute", shift_in(2, -0.2))
    add("doğu duvarı 20 cm dışa: köşelerdeki duvarlar izler, kapının yeri ve kuzeydeki boşluğun yeri korunur; tek adım", [
        first, {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed",
                                                          "entities": {str(i): BY_ID[i] for i in (1, 2, 3, 6, 7)}}}])
    for name, inp in [
        ("Mesafe: doğu duvarının iç yüzü bölmeye 2,50 m (15 cm dışa)", distance_in(2, "left", 2.5)),
        ("Mesafe: doğu duvarının iç yüzü bölmeye 2,00 m (35 cm içe); kuzey duvarındaki boşluğun at'ı azalır", distance_in(2, "left", 2.0)),
        ("güney duvarı 30 cm güneye: iki köşe ve saplanan bölme izler, doğu duvarındaki boşluğun at'ı artar", shift_in(1, -0.3)),
        ("iki duvara saplanan bölme 50 cm batıya: iki ucu duvarları boyunca kayar, onlar yerinde", shift_in(5, 0.5)),
        ("kapalı dikdörtgen duvarın sağ kenarı 50 cm dışa: iki köşe bitişik kenarlar boyunca", shift_in(8, -0.5, edge=1)),
        ("kapalı dikdörtgen duvarın alt kenarı 40 cm içe", shift_in(8, 0.4, edge=0)),
        ("tek başına duvar 30 cm: iki ucu dik kayar", shift_in(9, 0.3)),
        ("saplanan duvar 50 cm: ucu saplandığı duvar boyunca kayar, o yerinde", shift_in(11, 0.5)),
        ("içine duvar saplanan duvar 40 cm kuzeye: saplanan duvarın ucu izler", shift_in(10, 0.4)),
        ("aynı doğrultudaki iki duvara dik duran duvar: ucu o doğru boyunca kayar", shift_in(19, 0.5)),
        ("L duvarın ilk kenarı 50 cm: ucu dik, köşesi ikinci kenar boyunca", shift_in(21, 0.5, edge=0)),
        ("L duvarın ikinci kenarı 50 cm: köşesi ilk kenar boyunca, ucu dik", shift_in(21, 0.5, edge=1)),
        ("başka kattaki köşe duvarı izlemez: uç dik kayar", shift_in(24, 0.3)),
        ("birleşimi yok (none) olan uç dik kayar, köşedeki duvar izlemez", shift_in(28, 0.4)),
        ("sola hizalı duvarın sol yanındaki mesafe yüzünden ölçülür: 0,90 m'den 0,60 m'ye", distance_in(30, "left", 0.6)),
        ("Mesafe sağ yanda: sola hizalı duvarın sağ yüzünden 1,70 m'den 2,00 m'ye", distance_in(30, "right", 2.0)),
    ]:
        s, _ = step("execute", inp)
        add(name, [s])
    s, _ = step("plan", shift_in(2, -0.2))
    add("plan duvarı ve izleyenleri yazmadan gösterir", [s])
    s, _ = step("validate", shift_in(2, -0.2))
    add("doğrula yazmaz", [s])
    for name, inp, nf in [
        ("uç uca aynı doğrultudaki duvara bağlı duvar reddedilir", shift_in(12, 0.3), None),
        ("ucunda iki duvar birleşen duvar reddedilir", shift_in(14, 0.3), None),
        ("yay duvar reddedilir", shift_in(20, 0.3), None),
        ("komşu kenarı ters çeviren kaydırma reddedilir", shift_in(2, 6.0), None),
        ("izleyen duvar kilitli katmanda: hiçbiri yazılmaz", shift_in(22, -0.3), None),
        ("iki duvar birden reddedilir", {"uids": [u(2), u(4)], "operation": {"kind": "shift", "by": 0.2}}, None),
        ("olmayan kenar reddedilir", shift_in(2, 0.2, edge=3), None),
        ("paralel duvarı olmayan yanda Mesafe reddedilir", distance_in(2, "right", 1.0), None),
        ("eksi mesafe reddedilir", distance_in(2, "left", -1.0), None),
        ("sıfır kaydırma yazılmaz", shift_in(2, 0.0), None),
        ("mesafe zaten bu: yazılmaz", distance_in(2, "left", 2.35), None),
        ("sonsuz kaydırma reddedilir", shift_in(2, 0.2), {"operation.by": "NaN"}),
        ("duvar olmayan nesne reddedilir", shift_in(40, 0.2), None),
    ]:
        s, _ = step("execute", inp, non_finite=nf)
        add(name, [s])
    return c


def distances_answer(state, ids, edge=None):
    out = []
    for i in ids:
        k = 0 if edge is None else edge
        g = gaps(state, i, k)
        item = {"uid": u(i), "edge": k}
        for side, key in ((0, "left"), (1, "right")):
            if g[side] is not None:
                x = g[side]
                item[key] = {"distance": x["distance"], "uid": u(x["wall"]), "edge": x["edge"],
                             "face": "left" if x["left"] else "right", "from": P(*x["from"]), "to": P(*x["to"])}
        out.append(item)
    return {"walls": out, "revision": "$current"}


def distances_cases():
    state = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    c = []

    def ask(name, inp, op="execute"):
        edge = inp.get("edge")
        named = wc.named(inp, "Ölçülecek duvar verilmedi.")
        if isinstance(named, dict):
            result = named
        else:
            result = None
            for at, i in named:
                if state[i]["kind"] != "wall":
                    result = refusal("not_a_wall", "Nesne duvar değil; duvarın mesafeleri yalnız duvarlarda ölçülür.", f"uids[{at}]")
                    break
                try:
                    frame(state[i], 0 if edge is None else edge)
                except Refused as r:
                    result = refusal(r.code, r.message, "edge")
                    break
            if result is None:
                answer = distances_answer(state, [i for _, i in named], edge)
                result = {"status": "completed", "output": None if op == "validate" else answer, "warnings": []}
        c.append({"name": name, "steps": [{"op": op, "input": inp, "result": result, "within": 1e-9, "expect": before()}]})
    ask("doğu duvarı: iç yanda bölmeye 2,35 m, dış yanda duvar yok", {"uids": [u(2)]})
    ask("batı duvarı: iç yanda bölmenin sol yüzüne 2,35 m", {"uids": [u(4)]})
    ask("bölme: iki yanında odanın duvarlarına 2,35 m", {"uids": [u(5)]})
    ask("güney duvarı: dik bölme sayılmaz, kuzey duvarına 3,80 m", {"uids": [u(1)]})
    ask("kapalı dikdörtgen duvarın sağ kenarı kendi sol kenarına 4,80 m", {"uids": [u(8)], "edge": 1})
    ask("kapalı dikdörtgen duvarın alt kenarı: içte üst kenarına 3,80 m, dışta A odasının kuzey duvarına 5,80 m", {"uids": [u(8)], "edge": 0})
    ask("tek başına duvarın iki yanı boş", {"uids": [u(9)]})
    ask("sola hizalı duvar: yüzlerinden iki yana 0,90 m ve 1,70 m (örtüşmenin ortasında)", {"uids": [u(30)]})
    ask("eşit uzaklıkta iki duvar: çizimde önce gelen", {"uids": [u(33)]})
    ask("boyunca örtüşmeyen yandaki duvar atlanır, ötesindeki ölçülür", {"uids": [u(36)]})
    ask("0,01° eğik duvar paralel sayılmaz: o yan boş", {"uids": [u(38)]})
    ask("başka kattaki paralel duvar atlanır, aynı kattaki ölçülür", {"uids": [u(24)]})
    ask("L duvarın ikinci kenarı", {"uids": [u(21)], "edge": 1})
    ask("iki duvar birden", {"uids": [u(2), u(4)]})
    ask("plan aynı cevabı verir", {"uids": [u(2)]}, op="plan")
    ask("doğrula cevap vermez, yazmaz", {"uids": [u(2)]}, op="validate")
    ask("yay duvar reddedilir", {"uids": [u(20)]})
    ask("olmayan kenar reddedilir", {"uids": [u(2)], "edge": 2})
    ask("duvar olmayan nesne reddedilir", {"uids": [u(40)]})
    ask("kimliksiz istek reddedilir", {"uids": []})
    ask("geçersiz kimlik reddedilir", {"uids": ["D-1"]})
    ask("çizimde olmayan kimlik reddedilir", {"uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]})
    return c


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": SETUP, "cases": cases}


NOTE = ("ADR 0710. Beklenenler scripts/fixtures/wall_shift_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
        "birleşmeyen duvarların uyarıları scripts/fixtures/wall_join_cases.py'nin kesirli çözücüsünden. Açıklıkların at'ı pervazlarının "
        "ortasından yeni yola okunur, pervazları duvardan yeniden yazılır.")


def files():
    return {
        "cad.walls.distances.json": document(
            "cad.walls.distances", "Duvarın mesafeleri: iki yandaki en yakın paralel duvara net mesafe; kat, örtüşme, hiza; retler",
            NOTE, distances_cases()),
        "cad.walls.edit.shift.json": document(
            "cad.walls.edit", "Kaydır ve Mesafe: kenar kendine dik kayar, köşeler, T'ler ve açıklıklar izler; retler", NOTE, edit_cases()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{name}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
