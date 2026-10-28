#!/usr/bin/env python3
"""ADR 0701's shared cases: an opening's temporary distances along its wall's face (`cad.openings.distances`) and the
`distance` edit of `cad.openings.edit`, from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.openings.distances.json and cad.openings.edit.distance.json; `--check` compares and
writes nothing. The walls are straight and centred; a face is the path offset half the width to its left (`left`) or
right (`right`), its corners mitred by intersecting the offset lines (an open wall's ends offset square). An opening on
edge k of its wall (the edge its span along the path lies on) is measured on that edge's face segment [A, B], along
s = (p - A)·u, u = (B - A)/|B - A|:

- its jambs on the face: its place's points 0 and 2 on the left face, 1 and 3 on the right (s0 < s1);
- the references: the face segment's two ends (`end`), the jambs on the same face of the wall's other openings on that
  edge (their jambs' middle on the edge, within the wall's width of the face: `opening`), and where another wall's face
  segments cross [A, B] (`wall`);
- the start's reference is the nearest at or before s0 (1e-9 of slack), the end's the nearest at or after s1; the
  distances s0 - r and r - s1; on a tie the opening, then the wall, then the end, then the lower id;
- `distance` moves each opening by (r + d) - s0 (start) or (r - d) - s1 (end) along its wall, the reference fixed; the
  move's rules are Taşı's (one edge within its ends, clear of the other openings), its refusals Taşı's words.

ADR 0728 adds grids: a straight grid (no bulge) on the wall's level or on none, its line endless, crossing [A, B]
(not parallel within 0.001°; the crossing within 1e-9 of the segment), measured from the opening's middle
m = (s0 + s1) / 2: before when at or before it (1e-9), else after; each side's nearest, a tie to the grid first in the
drawing; `startGrid`, `endGrid` from the middle's point to the crossing. `distance` with `to: grid` moves the opening
by d - g (start) or g - d (end), the grid fixed.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
SEED = json.loads((ROOT / "fixtures/kcad/v2/dimension-sets.json").read_text())

ROOM = "0192f5a1-6000-7000-8000-000000000201"
PART = "0192f5a1-6000-7000-8000-000000000202"
DOOR = "0192f5a1-6000-7000-8000-000000000203"
WINDOW = "0192f5a1-6000-7000-8000-000000000204"
DOOR2 = "0192f5a1-6000-7000-8000-000000000205"
ARCHED = "0192f5a1-6000-7000-8000-000000000206"
LINE = "0192f5a1-6000-7000-8000-000000000207"
ORPHAN = "0192f5a1-6000-7000-8000-000000000208"
GONE = "0192f5a1-6000-7000-8000-0000000002ff"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
GRID_1 = "0192f5a1-6000-7000-8000-000000000211"
GRID_2 = "0192f5a1-6000-7000-8000-000000000212"
GRID_3 = "0192f5a1-6000-7000-8000-000000000213"
GRID_4 = "0192f5a1-6000-7000-8000-000000000214"
GRID_5 = "0192f5a1-6000-7000-8000-000000000215"
GRID_6 = "0192f5a1-6000-7000-8000-000000000216"
PARALLEL = 1.745_329_251_994_33e-5
QUERY = "cad.openings.distances"
EDIT = "cad.openings.edit"
TIE = 1e-9
PRIORITY = {"opening": 0, "wall": 1, "end": 2}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def num(x):
    """A number as Rust's `{}` writes an f64."""
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def unit(a, b):
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    length = math.hypot(dx, dy)
    return (dx / length, dy / length), length


# ── Walls: paths and faces ──────────────────────────────────────────────────────────────────────────────────────


def arc_of(a, b, bulge):
    """An arc edge: its centre, radius, start angle and signed sweep (counter-clockwise positive)."""
    (ux, uy), chord = unit(a, b)
    sweep = 4 * math.atan(bulge)
    radius = chord / (2 * math.sin(abs(sweep) / 2))
    d = chord / 2 * (1 - bulge * bulge) / (2 * bulge)
    centre = ((a["x"] + b["x"]) / 2 - uy * d, (a["y"] + b["y"]) / 2 + ux * d)
    start = math.atan2(a["y"] - centre[1], a["x"] - centre[0])
    return centre, radius, start, sweep


def path_edges(w):
    pts, n = w["pts"], len(w["pts"])
    bulges = w.get("bulges") or [0.0] * n
    count = n if w.get("closed") else n - 1
    out, total = [], 0.0
    for k in range(count):
        a, b = pts[k], pts[(k + 1) % n]
        (ux, uy), length = unit(a, b)
        e = {"a": a, "b": b, "u": (ux, uy), "length": length, "start": total, "arc": None}
        if bulges[k] != 0.0:
            e["arc"] = arc_of(a, b, bulges[k])
            e["length"] = e["arc"][1] * abs(e["arc"][3])
        out.append(e)
        total += e["length"]
    return out, total


def face(w, side):
    """The face polygon: one point per path vertex, the corners mitred (an open wall's ends square)."""
    pts, n, closed = w["pts"], len(w["pts"]), w.get("closed", False)
    h = w["width"] / 2 if side == "left" else -w["width"] / 2
    edges = n if closed else n - 1

    def offset_line(k):
        (ux, uy), _ = unit(pts[k], pts[(k + 1) % n])
        return (pts[k]["x"] - uy * h, pts[k]["y"] + ux * h), (ux, uy)

    out = []
    for i in range(n):
        if not closed and i == 0:
            (px, py), _ = offset_line(0)
            out.append(P(px, py))
            continue
        if not closed and i == n - 1:
            (px, py), _ = offset_line(n - 2)
            (ux, uy), length = unit(pts[n - 2], pts[n - 1])
            out.append(P(px + ux * length, py + uy * length))
            continue
        (p1, u1), (p2, u2) = offset_line((i - 1) % n), offset_line(i % edges)
        det = u1[0] * (-u2[1]) - u1[1] * (-u2[0])
        rx, ry = p2[0] - p1[0], p2[1] - p1[1]
        t = (rx * (-u2[1]) - ry * (-u2[0])) / det
        out.append(P(p1[0] + u1[0] * t, p1[1] + u1[1] * t))
    return out


def face_segments(w, side):
    poly, n = face(w, side), len(w["pts"])
    count = n if w.get("closed") else n - 1
    return [(poly[k], poly[(k + 1) % n]) for k in range(count)]


def cross(a, b, c, d):
    """Where segment c-d crosses a-b: the parameter along a-b (0 to 1), or None."""
    r = (b["x"] - a["x"], b["y"] - a["y"])
    q = (d["x"] - c["x"], d["y"] - c["y"])
    den = r[0] * q[1] - r[1] * q[0]
    if abs(den) <= 1e-12 * math.hypot(*r) * math.hypot(*q):
        return None
    w = (c["x"] - a["x"], c["y"] - a["y"])
    t = (w[0] * q[1] - w[1] * q[0]) / den
    v = (w[0] * r[1] - w[1] * r[0]) / den
    if -TIE <= t <= 1 + TIE and -TIE <= v <= 1 + TIE:
        return t
    return None


# ── The drawing ─────────────────────────────────────────────────────────────────────────────────────────────────


def wall(id, uid, pts, width, closed=False, layer="0"):
    e = {"kind": "wall", "id": id, "layerId": layer, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
         "justify": "center"}
    if closed:
        e["closed"] = True
    return e


def opening(id, uid, host, at, width, form="door", layer="0"):
    """An opening on `host` (a wall entity) centred `at` along its path, its jambs on the faces."""
    edges, _ = path_edges(host)
    s0, s1 = at - width / 2, at + width / 2
    return {"kind": "opening", "id": id, "layerId": layer, "attrs": {}, "host": None, "at": float(at),
            "width": float(width), "form": form, "place": jambs(host, s0, s1, edges)}


def jambs(w, s0, s1, edges=None):
    edges = edges or path_edges(w)[0]
    _, total = path_edges(w)
    tol = 1e-9 * max(total, 1.0)
    e = next((e for e in edges if s0 >= e["start"] - tol and s1 <= e["start"] + e["length"] + tol), None)
    if e is None:
        return None
    t = w["width"] / 2

    def at(s, h):
        d = s - e["start"]
        if e["arc"]:
            # Left of a counter-clockwise arc is its centre's side.
            (cx, cy), r, start, sweep = e["arc"]
            sign = 1.0 if sweep > 0 else -1.0
            phi = start + sign * d / r
            rr = r - sign * h
            return P(cx + rr * math.cos(phi), cy + rr * math.sin(phi))
        # The core's order of operations (opening_command_cases.py's `Line.at` and `beside`): the point at the edge's
        # fraction, then the normal of the edge's unit tangent; the stored jambs are then bit for bit the document's.
        # A closed wall's position goes round first, as the core's `norm_s` does: ((s mod L) + L) mod L, which can
        # move s by an ulp (2.025 on a 20 m room comes back as 2.0249999999999986); an open wall's is clamped.
        q = math.fmod(math.fmod(s, total) + total, total) if w.get("closed") else max(0.0, min(total, s))
        a, b = e["a"], e["b"]
        ln = e["length"]
        t = min(1.0, (q - e["start"]) / ln)
        tx, ty = (b["x"] - a["x"]) / ln, (b["y"] - a["y"]) / ln
        px, py = a["x"] + (b["x"] - a["x"]) * t, a["y"] + (b["y"] - a["y"]) * t
        return P(px - ty * h, py + tx * h)
    return [at(s0, t), at(s0, -t), at(s1, t), at(s1, -t)]


def drawing(entities, uids, locked=False):
    d = copy.deepcopy(SEED)
    d.pop("dimensionSets", None)
    d["name"] = "Açıklıkların mesafeleri"
    if locked:
        d["layers"][0]["locked"] = True
    d["entities"] = entities
    d["uids"] = uids
    return d


def room(rotate=0.0, origin=(0.0, 0.0), arched=False, locked=False, orphan=False):
    """An 8 × 6 m room of one closed wall (25 cm, counter-clockwise), a 10 cm wall teeing into its bottom edge at
    x = 4, a door at 2 m and a window at 6 m on the bottom edge, a door in the middle of the right edge; optionally
    turned `rotate` degrees about the origin and moved to `origin`, its top edge an arc."""
    c, s = math.cos(math.radians(rotate)), math.sin(math.radians(rotate))

    def at(x, y):
        return (origin[0] + c * x - s * y, origin[1] + s * x + c * y)
    r = wall(1, ROOM, [at(0, 0), at(8, 0), at(8, 6), at(0, 6)], 0.25, closed=True)
    if arched:
        r["bulges"] = [0.0, 0.0, 0.25, 0.0]
    p = wall(2, PART, [at(4, 0), at(4, 3)], 0.1)
    door = opening(3, DOOR, r, 2.0, 0.9)
    window = opening(4, WINDOW, r, 6.0, 1.2, form="window")
    door2 = opening(5, DOOR2, r, 11.0, 0.9)
    entities, uids = [r, p, door, window, door2], [ROOM, PART, DOOR, WINDOW, DOOR2]
    entities.append({"kind": "line", "id": 6, "layerId": "0", "attrs": {}, "a": P(*at(0, -3)), "b": P(*at(8, -3))})
    uids.append(LINE)
    if arched:
        entities.append(opening(7, ARCHED, r, 18.0, 0.9, form="window"))
        uids.append(ARCHED)
    for e in entities:
        if e["kind"] == "opening":
            e["host"] = ROOM
    if orphan:
        # A door whose wall is gone: its place kept, broken (ADR 0341). Its id is its place in the file, as the KCAD
        # writer requires, so the case also runs in Python.
        o = opening(len(entities) + 1, ORPHAN, r, 26.0, 0.9)
        o["host"], o["broken"] = GONE, "hostGone"
        entities.append(o)
        uids.append(ORPHAN)
    return drawing(entities, uids, locked)


def gridded(rotate=0.0, origin=(0.0, 0.0)):
    """The room with grids across its bottom edge at x 1, 2 (through the door's middle) and 6.5, one of the ground
    floor at x 1.8, one of an arc at x 2.2, one along the edge (parallel) and one past the room's west end."""
    d = room(rotate=rotate, origin=origin)
    c, s_ = math.cos(math.radians(rotate)), math.sin(math.radians(rotate))

    def at(x, y):
        return P(origin[0] + c * x - s_ * y, origin[1] + s_ * x + c * y)

    def grid(id, uid, a, b, name, **extra):
        d["entities"].append({"kind": "grid", "id": id, "layerId": "0", "attrs": {}, "pts": [at(*a), at(*b)],
                              "name": name, **extra})
        d["uids"].append(uid)
    grid(7, GRID_1, (1.0, -2), (1.0, 8), "1")
    grid(8, GRID_2, (2.0, -2), (2.0, 8), "2")
    grid(9, GRID_3, (6.5, -2), (6.5, 8), "3")
    grid(10, GRID_4, (1.8, -2), (1.8, 8), "4", arch={"level": "zemin"})
    grid(11, GRID_5, (2.2, -2), (2.2, 8), "5", bulges=[0.2, 0.0])
    grid(12, GRID_6, (-3.0, -1.0), (12.0, -1.0), "A")
    return d


def open_wall():
    """A 10 m open wall, a window 3 m from its start, a door in its middle; the window 4 m from the door's start
    jamb's reference."""
    w = wall(1, ROOM, [(0, 0), (10, 0)], 0.2)
    window = opening(2, WINDOW, w, 3.0, 1.0, form="window")
    door = opening(3, DOOR, w, 6.5, 0.9)
    for e in (window, door):
        e["host"] = ROOM
    return drawing([w, window, door], [ROOM, WINDOW, DOOR])


# ── The rule ────────────────────────────────────────────────────────────────────────────────────────────────────


def by_uid(d):
    return {u: e for u, e in zip(d["uids"], d["entities"])}


def edge_of(w, o):
    edges, total = path_edges(w)
    s0, s1 = o["at"] - o["width"] / 2, o["at"] + o["width"] / 2
    tol = 1e-9 * max(total, 1.0)
    for k, e in enumerate(edges):
        if s0 >= e["start"] - tol and s1 <= e["start"] + e["length"] + tol:
            return k
    return None


def distances(d, uid, side):
    """(edge, start, end) of the opening `uid` on its wall's `side` face; a side None without a reference; or a
    refusal."""
    objects = by_uid(d)
    o = objects[uid]
    if o.get("broken") or o["host"] not in objects:
        return refusal("opening_host_gone", "Açıklığın duvarı yok; geçici ölçü duvarın yüzünde ölçülür. Önce bir duvara taşıyın (Başka duvara).", "uids[0]")
    w = objects[o["host"]]
    k = edge_of(w, o)
    if (w.get("bulges") or [0.0] * len(w["pts"]))[k] != 0.0:
        return refusal("invalid_opening", "Açıklık duvarın yay kenarında; geçici ölçü yalnız düz kenarda ölçülür.", "uids[0]")
    a, b = face_segments(w, side)[k]
    (ux, uy), length = unit(a, b)
    along = lambda p: (p["x"] - a["x"]) * ux + (p["y"] - a["y"]) * uy  # noqa: E731
    off = lambda p: (p["x"] - a["x"]) * -uy + (p["y"] - a["y"]) * ux  # noqa: E731
    pick = (0, 2) if side == "left" else (1, 3)
    j0, j1 = o["place"][pick[0]], o["place"][pick[1]]
    s0, s1 = sorted((along(j0), along(j1)))
    if along(j0) > along(j1):
        j0, j1 = j1, j0
    candidates = [("end", None, 0.0), ("end", None, length)]
    for u2, e in objects.items():
        if e["kind"] == "opening" and u2 != uid and e["host"] == o["host"] and e.get("place"):
            mid = P(sum(p["x"] for p in e["place"]) / 4, sum(p["y"] for p in e["place"]) / 4)
            if -TIE <= along(mid) <= length + TIE and abs(off(mid)) <= w["width"] + TIE:
                for i in pick:
                    candidates.append(("opening", u2, along(e["place"][i])))
        if e["kind"] == "wall" and u2 != o["host"]:
            for f in ("left", "right"):
                for c, dd in face_segments(e, f):
                    t = cross(a, b, c, dd)
                    if t is not None:
                        candidates.append(("wall", u2, t * length))

    def best(cands, before):
        if not cands:
            return None
        edge = max(c[2] for c in cands) if before else min(c[2] for c in cands)
        tied = [c for c in cands if abs(c[2] - edge) <= TIE]
        return min(tied, key=lambda c: (PRIORITY[c[0]], c[1] or "", -c[2] if before else c[2]))

    def described(c, jamb, s, before):
        if c is None:
            return None
        ref = {"kind": c[0]}
        if c[1]:
            ref["uid"] = c[1]
        to = P(a["x"] + ux * c[2], a["y"] + uy * c[2])
        return {"distance": s - c[2] if before else c[2] - s, "from": jamb, "to": to, "reference": ref}
    start = best([c for c in candidates if c[2] <= s0 + TIE], True)
    end = best([c for c in candidates if c[2] >= s1 - TIE], False)
    # Grids crossing the face, from the opening's middle (ADR 0728 §1).
    mid = (s0 + s1) / 2
    grids = [None, None]
    level = (w.get("arch") or {}).get("level")
    for u2, e in objects.items():
        if e["kind"] != "grid" or len(e["pts"]) != 2 or any(x != 0.0 for x in (e.get("bulges") or [])):
            continue
        if (e.get("arch") or {}).get("level") not in (None, level):
            continue
        (vx, vy), _ = unit(e["pts"][0], e["pts"][1])
        den = ux * vy - uy * vx
        if abs(den) <= PARALLEL:
            continue
        cx, cy = e["pts"][0]["x"] - a["x"], e["pts"][0]["y"] - a["y"]
        t = (cx * vy - cy * vx) / den
        if not (-TIE <= t <= length + TIE):
            continue
        before = t <= mid + TIE
        dd = max(0.0, mid - t) if before else t - mid
        side = 0 if before else 1
        if grids[side] is None or dd < grids[side][0] - TIE:
            grids[side] = (dd, u2, t)
    middle = P(a["x"] + (b["x"] - a["x"]) * (mid / length), a["y"] + (b["y"] - a["y"]) * (mid / length))

    def grid_described(g):
        if g is None:
            return None
        return {"distance": g[0], "from": middle, "to": P(a["x"] + ux * g[2], a["y"] + uy * g[2]),
                "reference": {"kind": "grid", "uid": g[1]}}
    return (k, described(start, j0, s0, True), described(end, j1, s1, False), (s0, s1),
            grid_described(grids[0]), grid_described(grids[1]))


def queried(d, uids, side="left"):
    out = []
    for at, u in enumerate(uids):
        r = distances(d, u, side)
        if isinstance(r, dict):
            r["error"]["path"] = f"uids[{at}]"
            return r
        k, start, end, _, start_grid, end_grid = r
        one = {"uid": u, "edge": k, "face": side}
        if start:
            one["start"] = start
        if end:
            one["end"] = end
        if start_grid:
            one["startGrid"] = start_grid
        if end_grid:
            one["endGrid"] = end_grid
        out.append(one)
    return completed({"openings": out, "revision": "$current"})


def span(o):
    return (o["at"] - o["width"] / 2, o["at"] + o["width"] / 2)


def overlap(a, b):
    tol = 1e-9 * max(abs(a[1]), abs(b[1]), 1.0)
    return a[0] < b[1] - tol and b[0] < a[1] - tol


def stored(o):
    return {k: o[k] for k in ("kind", "id", "layerId", "attrs", "host", "at", "width", "form", "place", "broken") if k in o}


def moved(d, uids, side, face_side, value, to="face"):
    """The `distance` edit: (result, the drawing after) or (refusal, None)."""
    if not math.isfinite(value):
        return refusal("not_finite", "Mesafe sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "operation.distance"), None
    if value < 0:
        return refusal("invalid_opening", f"Mesafe {num(value)} m; sıfır ya da daha büyük olmalı.", "operation.distance"), None
    objects = by_uid(d)
    after = copy.deepcopy(d)
    moved_ = by_uid(after)
    for at, u in enumerate(uids):
        r = distances(d, u, face_side)
        if isinstance(r, dict):
            r["error"]["path"] = f"uids[{at}]"
            return r, None
        _, start, end, (s0, s1), start_grid, end_grid = r
        if to == "grid":
            ref = start_grid if side == "start" else end_grid
        else:
            ref = start if side == "start" else end
        if ref is None:
            where = "başında" if side == "start" else "sonunda"
            if to == "grid":
                return refusal("invalid_opening", f"Açıklığın {where} bu yüzü kesen bir aks yok; aksa mesafe ölçülemiyor. Öbür yanı seçin ya da yüze ölçün.", f"uids[{at}]"), None
            return refusal("invalid_opening", f"Açıklığın {where} bu yüzde referans yok: yüzün ucu açıklığın gerisinde kalıyor. Öbür yüzü ya da yanı seçin.", f"uids[{at}]"), None
        # The reference stays: the jamb goes to it plus (start) or less (end) the distance.
        by = (value - ref["distance"]) if side == "start" else (ref["distance"] - value)
        o = moved_[u]
        o["at"] = objects[u]["at"] + by
        w = objects[o["host"]]
        edges, total = path_edges(w)
        e = edges[edge_of(w, objects[u])]
        s = span(o)
        tol = 1e-9 * max(total, 1.0)
        if s[0] < e["start"] - tol or s[1] > e["start"] + e["length"] + tol:
            return refusal("opening_off_wall", f"Açıklık bu mesafeyle bulunduğu duvar kenarının dışına taşıyor: duvar boyunca {s[0]:.3f}–{s[1]:.3f} m'de, "
                           f"kenar {e['start']:.3f}–{e['start'] + e['length']:.3f} m. Daha küçük bir mesafe verin.", f"uids[{at}]"), None
        o["place"] = jambs(w, *s)
    for at, u in enumerate(uids):
        a = span(moved_[u])
        for u2, e in moved_.items():
            if e["kind"] == "opening" and u2 != u and e["host"] == moved_[u]["host"] and overlap(a, span(e)):
                b = span(e)
                return refusal("openings_overlap", f"Açıklık ({a[0]:.3f}–{a[1]:.3f} m) aynı duvardaki başka bir açıklığa ({b[0]:.3f}–{b[1]:.3f} m) "
                               "biniyor. Açıklığı kaydırın ya da daraltın.", f"uids[{at}]"), None
    if d["layers"][0]["locked"]:
        return refusal("layer_locked", "“0” katmanı kilitli; üzerindeki açıklık düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"), None
    return completed({"openings": [stored(moved_[u]) for u in uids], "revision": "$current"}), after


# ── Cases ───────────────────────────────────────────────────────────────────────────────────────────────────────


def case(name, d, steps):
    return {"name": name, "setup": d, "steps": steps}


def query_step(d, inp, result):
    return {"op": "execute", "input": inp, "result": result, "within": 1e-9,
            "expect": {"ids": [e["id"] for e in d["entities"]], "revision": "same"}}


def query_cases():
    out = []
    d = room()
    for u, name in ((DOOR, "kapı: başta iç köşe, sonda ara duvarın yüzü"), (WINDOW, "pencere: başta ara duvarın öbür yüzü, sonda köşe"),
                    (DOOR2, "sağ kenardaki kapı: iki köşe")):
        out.append(case("İç yüz, " + name, d, [query_step(d, {"uids": [u]}, queried(d, [u]))]))
    for u, name in ((DOOR, "kapı: başta dış köşe, sonda pencere"), (WINDOW, "pencere: başta kapı, sonda dış köşe"),
                    (DOOR2, "sağ kenardaki kapı, dış yüz")):
        out.append(case("Dış yüz, " + name, d, [query_step(d, {"uids": [u], "face": "right"}, queried(d, [u], "right"))]))
    out.append(case("Üç açıklık birlikte, sırasıyla", d, [query_step(d, {"uids": [DOOR, WINDOW, DOOR2]}, queried(d, [DOOR, WINDOW, DOOR2]))]))
    out.append(case("Plan sorgunun aynısı", d, [dict(query_step(d, {"uids": [DOOR]}, queried(d, [DOOR])), op="plan")]))
    out.append(case("Doğrulama yazmaz", d, [{"op": "validate", "input": {"uids": [DOOR]}, "result": {"status": "completed", "output": None, "warnings": []},
                                            "expect": {"ids": [1, 2, 3, 4, 5, 6], "revision": "same"}}]))
    for rot, org, name in ((30.0, (0.0, 0.0), "30° dönük oda"), (0.0, (500000.0, 4400000.0), "büyük koordinatlarda oda"),
                           (-73.0, (412345.678, 4523456.789), "dönük ve büyük koordinatlı oda")):
        d2 = room(rotate=rot, origin=org)
        out.append(case(name + ", iç yüz", d2, [query_step(d2, {"uids": [DOOR, WINDOW]}, queried(d2, [DOOR, WINDOW]))]))
        out.append(case(name + ", dış yüz", d2, [query_step(d2, {"uids": [DOOR], "face": "right"}, queried(d2, [DOOR], "right"))]))
    w = open_wall()
    out.append(case("Açık duvar: pencere başta duvar ucundan, sonda kapıya", w, [query_step(w, {"uids": [WINDOW]}, queried(w, [WINDOW]))]))
    out.append(case("Açık duvar: kapı başta pencereden, sonda duvar ucuna", w, [query_step(w, {"uids": [DOOR], "face": "right"}, queried(w, [DOOR], "right"))]))
    arc = room(arched=True)
    out.append(case("Yay kenar olmayan açıklık yay duvarda da ölçülür", arc, [query_step(arc, {"uids": [DOOR]}, queried(arc, [DOOR]))]))
    out.append(case("Ret: yay kenardaki açıklık", arc, [query_step(arc, {"uids": [DOOR, ARCHED]}, queried(arc, [DOOR, ARCHED]))]))
    orphan = room(orphan=True)
    out.append(case("Ret: duvarı olmayan açıklık", orphan, [{"op": "execute", "input": {"uids": [DOOR, ORPHAN]}, "result": queried(orphan, [DOOR, ORPHAN]),
                                                            "expect": {"ids": [e["id"] for e in orphan["entities"]], "revision": "same"}}]))
    g = gridded()
    out.append(case("Aks: kapının ortası aksın üstünde (başta 0), sonda 4,50 m; başka kattaki, yay ve paralel aks sayılmaz", g,
                    [query_step(g, {"uids": [DOOR]}, queried(g, [DOOR]))]))
    out.append(case("Aks: pencerenin ortasından başta 4 m, sonda 0,50 m", g, [query_step(g, {"uids": [WINDOW]}, queried(g, [WINDOW]))]))
    out.append(case("Aks: dış yüzde de ortadan aynı mesafeler", g,
                    [query_step(g, {"uids": [DOOR, WINDOW], "face": "right"}, queried(g, [DOOR, WINDOW], "right"))]))
    out.append(case("Aks: sağ kenardaki kapının yüzünü aks kesmez", g, [query_step(g, {"uids": [DOOR2]}, queried(g, [DOOR2]))]))
    g2 = gridded(rotate=30.0, origin=(500000.0, 4400000.0))
    out.append(case("Aks: dönük ve büyük koordinatlı odada aynı mesafeler", g2,
                    [query_step(g2, {"uids": [DOOR, WINDOW]}, queried(g2, [DOOR, WINDOW]))]))
    ids = [1, 2, 3, 4, 5, 6]
    for name, inp, res in (
        ("Ret: açıklık verilmedi", {"uids": []}, refusal("no_entities", "Ölçülecek açıklık verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")),
        ("Ret: geçersiz kimlik", {"uids": ["kapı"]}, refusal("invalid_uid", "“kapı” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]")),
        ("Ret: çizimde olmayan kimlik", {"uids": [MISSING]}, refusal("entity_not_found", f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]")),
        ("Ret: açıklık olmayan nesne", {"uids": [LINE]}, refusal("not_an_opening", "Nesne kapı, pencere ya da boşluk değil; geçici ölçü yalnız açıklıkların mesafesini verir.", "uids[0]")),
    ):
        out.append(case(name, d, [{"op": "execute", "input": inp, "result": res, "expect": {"ids": ids, "revision": "same"}}]))
    return out


def edit_step(d, inp, op="execute"):
    o = inp["operation"]
    result, after = moved(d, inp["uids"], o["side"], o.get("face", "left"), o["distance"], o.get("to", "face"))
    ids = [e["id"] for e in d["entities"]]
    s = {"op": op, "input": inp, "result": result, "within": 1e-9}
    if after is None or op != "execute":
        if op == "validate" and after is not None:
            s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": ids, "revision": "same"}
        return s, None
    changed = {str(e["id"]): e for u, e in zip(after["uids"], after["entities"]) if u in inp["uids"]}
    s["expect"] = {"ids": ids, "entities": changed, "canUndo": True, "dirty": True, "revision": "changed"}
    return s, after


def edit_cases():
    out = []
    d = room()

    def one(name, d, uids, side, value, face="left", undo=False, op="execute", to=None):
        inp = {"uids": uids, "operation": {"kind": "distance", "side": side, "face": face, "distance": value}}
        if to is not None:
            inp["operation"]["to"] = to
        step, after = edit_step(d, inp, op)
        steps = [step]
        if undo and after is not None:
            before = {str(e["id"]): e for u, e in zip(d["uids"], d["entities"]) if u in uids}
            steps.append({"op": "undo", "expect": {"ids": step["expect"]["ids"], "entities": before, "canUndo": False, "canRedo": True}})
            steps.append({"op": "redo", "expect": {"ids": step["expect"]["ids"], "entities": step["expect"]["entities"], "canUndo": True}})
        out.append(case(name, d, steps))

    one("Kapı iç köşeden 1 m, tek geri al ve yinele", d, [DOOR], "start", 1.0, undo=True)
    one("Kapı ara duvarın yüzüne 50 cm", d, [DOOR], "end", 0.5)
    one("Kapı dış köşeden 2 m (dış yüz)", d, [DOOR], "start", 2.0, face="right")
    one("Kapı pencereye 30 cm (dış yüz)", d, [DOOR], "end", 0.3, face="right")
    one("Pencere ara duvardan 60 cm", d, [WINDOW], "start", 0.6)
    one("Pencere köşeye 1 m", d, [WINDOW], "end", 1.0)
    one("Sağ kenardaki kapı köşeden 40 cm", d, [DOOR2], "start", 0.4)
    one("Sıfır mesafe: pervaz köşeye değer", d, [DOOR], "start", 0.0)
    one("Pervazlar değebilir: kapı pencereye sıfır (dış yüz)", d, [DOOR], "end", 0.0, face="right")
    one("Plan yazmadan yeni yeri gösterir", d, [DOOR], "start", 1.0, op="plan")
    one("Doğrulama yazmaz", d, [DOOR], "start", 1.0, op="validate")
    for rot, org, name in ((30.0, (0.0, 0.0), "30° dönük oda"), (0.0, (500000.0, 4400000.0), "büyük koordinatlarda oda")):
        d2 = room(rotate=rot, origin=org)
        one(name + ": kapı iç köşeden 1 m", d2, [DOOR], "start", 1.0)
    w = open_wall()
    one("Açık duvar: pencere duvar ucundan 50 cm", w, [WINDOW], "start", 0.5)
    one("Açık duvar: kapı pencereye 1 m (sağ yüz)", w, [DOOR], "start", 1.0, face="right")
    one("Ret: pencereye binen kapı", d, [DOOR], "start", 4.5)
    one("Ret: köşeyi aşıp öbür kenara geçecek kapı", d, [DOOR2], "end", 6.0)
    one("Ret: duvar ucunu aşan pencere", open_wall(), [WINDOW], "start", -0.0 + 9.5)
    one("Ret: negatif mesafe", d, [DOOR], "start", -0.1)
    one("Ret: sonsuz mesafe", d, [DOOR], "start", float("inf"))
    arc = room(arched=True)
    one("Ret: yay kenardaki açıklık", arc, [ARCHED], "start", 1.0)
    one("Yay duvarın düz kenarındaki kapı taşınır", arc, [DOOR], "start", 1.0)
    locked = room(locked=True)
    one("Ret: kilitli katmandaki açıklık", locked, [DOOR], "start", 1.0)
    one("Ret: duvarı olmayan açıklık taşınmaz", room(orphan=True), [ORPHAN], "start", 1.0)
    g = gridded()
    one("Aksa mesafe: kapının ortası aksın 50 cm ötesine, aks yerinde", g, [DOOR], "start", 0.5, to="grid", undo=True)
    one("Aksa mesafe: pencerenin ortası sonraki aksın üstüne", g, [WINDOW], "end", 0.0, to="grid")
    one("Aksa mesafe dış yüzde: pencere önceki aksa 3,50 m", g, [WINDOW], "start", 3.5, face="right", to="grid")
    g2 = gridded(rotate=30.0, origin=(500000.0, 4400000.0))
    one("Aksa mesafe dönük ve büyük koordinatlı odada", g2, [DOOR], "end", 4.0, to="grid")
    one("Ret: aksı olmayan yanda aksa mesafe", g, [DOOR2], "start", 1.0, to="grid")
    one("Ret: aksa mesafe pencereye bindirir", g, [DOOR], "end", 0.3, to="grid")
    return out


def finite_text(cases):
    """JSON cannot hold an infinity: the step names the field the runner sets to it."""
    for c in cases:
        for s in c["steps"]:
            o = s.get("input", {}).get("operation", {})
            if isinstance(o.get("distance"), float) and not math.isfinite(o["distance"]):
                o["distance"] = 0.0
                s["nonFinite"] = {"operation.distance": "Infinity"}
    return cases


def document(command, title, cases):
    assert len(cases) >= 20, (command, len(cases))
    common = copy.deepcopy(cases[0]["setup"])
    for c in cases:
        if c["setup"] == common:
            c.pop("setup")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 0701, 0728. Bağımsız yüz/pervaz ve aks hesabı (scripts/fixtures/opening_distance_cases.py); KentOS kodu çalıştırılmaz.",
            "setup": common, "cases": cases}


def files():
    return {
        "cad.openings.distances.json": document(QUERY, "Açıklığın geçici ölçüleri: yüz boyunca iki yanın mesafesi ve referansı", query_cases()),
        "cad.openings.edit.distance.json": document(EDIT, "Açıklığı mesafeyle taşı: referans sabit, tek adım; retler", finite_text(edit_cases())),
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
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
