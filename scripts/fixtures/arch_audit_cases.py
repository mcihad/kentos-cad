#!/usr/bin/env python3
"""ADR 0703's shared cases: `cad.arch.audit` (Mimari denetim), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.arch.audit.json; `--check` compares and writes nothing. The drawing is a flat of one
closed wall and four inner walls (one stops 4 cm short of another, two miss each other by 5 cm), an acute corner,
openings, spaces, tags, dimensions, a dimension set, a grid and columns, a view mark, a stair and a ramp; the stored
`broken` fields are the followers' (ADR 0341–0345, 0249, 0700), read as they are.

The rules, as ADR 0703 gives them:

- Walls, each level's apart (walls of different levels, or one without, never join: ADR 0639): the joins' issues are
  `wall_join_cases.py`'s exact solver's (its fix a `cad.walls.edit` `joins` input). A free end is an open wall's end
  not `none`, no joint's member, no other end on it and not inside or on another wall's body (between its faces, an
  open wall's ends capped square); its ray along its end edge's direction (out of the wall) meeting another wall's
  body outline within `maxGap` is `arch_wall_end_gap` (the nearest; a fix `extend` when the ray also crosses that
  wall's path); two free ends within `maxGap` of each other, neither with a gap, are `arch_wall_ends_apart`.
- Openings: stored `broken` (`arch_opening_broken`), no type (`tag_missing_type`); the place's four points' mean.
- Spaces: stored `broken` (`arch_space_broken`), area below `minArea` (`arch_space_small`), else 2·area/perimeter below
  `minWidth` (`arch_space_narrow`); area by the shoelace of every ring (holes off), the perimeter every ring's length;
  the point the first part's outer ring's centroid.
- Numbers: the tags check's duplicates (openings and spaces apart); tags whose object is gone (`tag_target_gone`,
  fix delete); a dimension set's stored `broken` (fix `detach`), a dimension outside every set with a broken reference
  (fix: those points freed); a column's (`detach`) and a view mark's (`detach`) stored `broken`; a wall, space,
  column, stair or ramp without a level, or with one the project lacks, while the project has levels.
- Grids (ADR 0725 §6): a label on more than one grid of a level (no level its own), each grid after the first said
  with the first (`arch_grid_label_duplicate`, at its middle); its fix deletes it when it lies on the first (the same
  ends, or swapped with its arc turned over, within a nanometre) and no column is tied to it, else names it the next
  label after its own that no grid of the level has and no earlier fix gave.
- A fix only when the object it writes is on an unlocked layer. Order: the groups, each group's codes, then x, then y.
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_join_cases as wj  # noqa: E402  (the joins' reference: exact fractions, straight walls)
from arch_command_cases import next_label  # noqa: E402  (the grid labels' rule, ADR 0343)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.audit.json"
COMMAND = "cad.arch.audit"
JOIN_TOLERANCE = 1e-3
DEFAULTS = {"maxGap": 0.10, "minArea": 0.10, "minWidth": 0.15}
GROUPS = ["walls", "openings", "spaces", "numbers", "tags", "dimensions", "grids", "columns", "viewmarks", "levels"]
CODES = {
    "walls": ["wall_join_overlap", "wall_join_crowded", "wall_join_acute", "wall_join_unresolved", "arch_wall_end_gap",
              "arch_wall_ends_apart"],
    "openings": ["arch_opening_broken", "tag_missing_type"],
    "spaces": ["arch_space_broken", "arch_space_small", "arch_space_narrow"],
    "numbers": ["tag_duplicate_number"],
    "tags": ["tag_target_gone"],
    "dimensions": ["arch_dimension_set_broken", "arch_dimension_broken"],
    "grids": ["arch_grid_label_duplicate"],
    "columns": ["arch_column_grid_gone"],
    "viewmarks": ["arch_viewmark_target_gone"],
    "levels": ["arch_level_missing"],
}
MODE_LABEL = {"auto": "Otomatik", "none": "Yok", "miter": "Gönye", "butt": "Alın", "through": "Geçen"}

# The followers' words (ADR 0341, 0342, 0343, 0345), each split at its first ". ": what is wrong, what resolves it.
OPENING_WORDS = {
    "hostGone": "Açıklığın duvarı yok. Açıklığı bir duvara taşıyın (Açıklığı taşı) ya da silin.",
    "offWall": "Açıklık duvarının ucundan ya da köşesinden taşıyor. Açıklığı duvar boyunca kaydırın ya da daraltın.",
    "overlap": "Açıklık aynı duvardaki başka bir açıklığa biniyor. Birini kaydırın ya da daraltın.",
}
SPACE_WORDS = {
    "lostEnclosure": "Mahali duvarlar artık kapatmıyor; sınırı son hâliyle kaldı. Duvarları kapatın ya da mahali yeniden bulun (Mahali düzenle › Duvarlardan bul).",
    "noFreeArea": "Kolonlar mahalın alanını tamamen kaplıyor; sınırı son hâliyle kaldı. Kolonları veya kolon alanı tercihini düzeltin.",
    "sourceUnavailable": "Seçilmiş mahal kaynağı yok veya artık uygun değil; sınır son geçerli hâliyle kaldı. Kaynakları geri getirin ya da açıkça yeniden seçin.",
    "invalidBoundarySource": "Mahalın kolon kaynağı okunamıyor; sınırı son hâliyle kaldı. Kaynak kolonun geometrisini düzeltin.",
}
COLUMN_WORDS = ("Kolonun akslarından biri yok ya da akslar kesişmiyor; kolon son yerinde kaldı. Aksı geri getirin ya da "
                "kolonu başka akslara bağlayın.")
VIEWMARK_WORDS = ("İşaretin hedefi (adlı görünüm ya da paftadaki pencere) yok; işaret son yazısıyla kaldı. Başvurular'dan "
                  "yeniden bağlayın ya da bağı kopartın.")
SET_WORDS = {
    "hostGone": "Ölçü dizisinin duvarı çizimde yok; ölçüler son yerlerinde duruyor.",
    "unsupportedWall": "Ölçü dizisinin duvar kenarı artık ölçülemiyor (pahlı köşe, yay ya da olmayan kenar); ölçüler son yerlerinde duruyor.",
    "openingUnplaced": "Ölçü dizisinin bir açıklığı duvarına yerleşmiyor; ölçüler son yerlerinde duruyor.",
}
POSSESSIVE = {"wall": "Duvarın", "space": "Mahalın", "column": "Kolonun", "stair": "Merdivenin", "ramp": "Rampanın",
              "polyline": "Mahal ayırıcının"}


def acc_place(a, b):
    """A column's place from its grids' labels: a lettered one first (ADR 0343 §3)."""
    lettered = lambda t: t[:1].isalpha()  # noqa: E731
    return f"{b}-{a}" if not lettered(a) and lettered(b) else f"{a}-{b}"


def split(words):
    what, rest = words.split(". ", 1)
    return what + ".", rest


def P(x, y):
    return {"x": float(x), "y": float(y)}


def num(x):
    """A number as Rust's `{}` writes an f64: integral values without a point, the others as the shortest repr."""
    if isinstance(x, float) and x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


# ── The drawing ─────────────────────────────────────────────────────────────────────────────────────────────────


def U(n):
    return f"0192f5a1-7030-7000-8000-{n:012x}"


GONE = {"wall": U(0xff01), "of": U(0xff02), "of2": U(0xff03), "set": U(0xff04), "grid": U(0xff05)}
SET = U(0x500)
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def layer(id, name, visible=True, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def base(id, layer_id, **extra):
    return {"id": id, "layerId": layer_id, "attrs": {}, **extra}


def wall(id, pts, width, closed=False, level="zemin", **extra):
    e = {"kind": "wall", **base(id, "duvar"), "pts": [P(*p) for p in pts], "width": width, "justify": "center", **extra}
    if closed:
        e["closed"] = True
    if level:
        e["arch"] = {"level": level}
    return e


def place_on(w, s0, s1):
    """The jambs of an opening from s0 to s1 along a straight-edged wall's path, centred: the start jamb on the left
    face, then the right, then the end jamb's."""
    pts = [(p["x"], p["y"]) for p in w["pts"]]
    n = len(pts)
    count = n if w.get("closed") else n - 1
    start = 0.0
    for k in range(count):
        a, b = pts[k], pts[(k + 1) % n]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if s0 >= start - 1e-9 and s1 <= start + length + 1e-9:
            h = w["width"] / 2

            # The core's order of operations (opening_command_cases.py's `Line.at` and `beside`).
            def at(s, side):
                t = (s - start) / length
                tx, ty = (b[0] - a[0]) / length, (b[1] - a[1]) / length
                px, py = a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t
                return P(px - ty * (h * side), py + tx * (h * side))
            return [at(s0, 1), at(s0, -1), at(s1, 1), at(s1, -1)]
        start += length
    raise ValueError("off the wall")


def opening(id, host_entity, host_uid, at, width, form="door", style="kentos-kapi-90", number=None, broken=None):
    e = {"kind": "opening", **base(id, "mimari"), "host": host_uid, "at": float(at), "width": float(width), "form": form,
         "place": place_on(host_entity, at - width / 2, at + width / 2)}
    if style:
        e["style"] = style
    if number:
        e["number"] = number
    if broken:
        e["broken"] = broken
    return e


def space(id, ring, number=None, name=None, level="zemin", layer_id="mimari", broken=None):
    e = {"kind": "space", **base(id, layer_id), "parts": [{"outer": {"pts": [P(*p) for p in ring]}}]}
    if number:
        e["number"] = number
    if name:
        e["name"] = name
    if level:
        e["arch"] = {"level": level}
    if broken:
        e["broken"] = broken
    return e


def vertex(uid, index, broken=None):
    r = {"uid": uid, "place": {"kind": "vertex", "part": 0, "ring": 0, "index": index}}
    if broken:
        r["broken"] = broken
    return r


def dimension(id, a, b, refs=None):
    e = {"kind": "dimension", **base(id, "olcu"), "a": P(*a), "b": P(*b), "height": 0.125, "offset": -0.5,
         "style": "linear", "angle": 0.0}
    if refs:
        e["refs"] = refs
    return e


def drawing():
    """The flat (ids, uids U(id)): see the module's note."""
    w1 = wall(1, [(0, 0), (10, 0), (10, 8), (0, 8)], 0.2, closed=True)
    ents = [
        w1,
        wall(2, [(0, 4), (6, 4)], 0.2),
        wall(3, [(5, 8), (5, 4.14)], 0.1),
        wall(4, [(7, 2), (8.5, 2)], 0.1),
        wall(5, [(8.53, 2.04), (8.53, 3.5)], 0.1),
        wall(6, [(20, 0), (24, 0)], 0.2),
        wall(7, [(24, 0), (19, 1.125)], 0.2),
        # Without a level: 4 cm from the room's outer face, but walls of different levels never join (ADR 0639).
        wall(28, [(14, 6), (10.14, 6)], 0.2, level=None),
        opening(8, w1, U(1), 2.0, 0.9, number="K01"),
        opening(9, w1, U(1), 6.0, 1.2, form="window", style=None, number="P01"),
        opening(10, w1, GONE["wall"], 8.5, 0.9, number="K02", broken="hostGone"),
        opening(11, w1, U(1), 12.0, 0.9, number="K01"),
        space(12, [(0.1, 0.1), (9.9, 0.1), (9.9, 3.9), (0.1, 3.9)], "101", "Salon"),
        space(13, [(0.1, 4.1), (4.95, 4.1), (4.95, 7.9), (0.1, 7.9)], "102", "Yatak odası", broken="lostEnclosure"),
        space(14, [(1, 10), (6, 10), (6, 10.05), (1, 10.05)], "103"),
        space(15, [(12, 10), (12.3, 10), (12.3, 10.25), (12, 10.25)], "104", "Şaft", layer_id="gizli"),
        space(16, [(20, 10), (23, 10), (23, 13), (20, 13)], "101", "Depo", level=None),
        {"kind": "tag", **base(17, "mimari"), "of": U(8), "at": P(2, -0.6), "text": "K01"},
        {"kind": "tag", **base(18, "kilitli"), "of": GONE["of"], "at": P(6, -1.5), "text": "K09", "broken": "targetGone"},
        {"kind": "tag", **base(19, "mimari"), "of": GONE["of2"], "at": P(31, -1), "text": "K10", "broken": "targetGone"},
        dimension(20, (0, -1), (10, -1), {"a": vertex(U(1), 0, "vertex_gone"), "b": vertex(U(1), 1)}),
        # The set's member: its references the set's, broken as loading leaves them (its wall is gone).
        dimension(21, (20, -2), (24, -2), {"a": vertex(GONE["wall"], 0, "object_gone"),
                                           "b": vertex(GONE["wall"], 1, "object_gone")}),
        {"kind": "grid", **base(22, "mimari"), "pts": [P(30, -2), P(30, 6)], "name": "A"},
        {"kind": "column", **base(23, "mimari"), "at": P(30, 0), "width": 0.4, "depth": 0.4,
         "grids": [U(22), GONE["grid"]], "arch": {"level": "zemin"}, "broken": "gridGone"},
        {"kind": "column", **base(24, "mimari"), "at": P(34, 0), "width": 0.4, "depth": 0.4},
        {"kind": "viewmark", **base(25, "mimari"), "form": "section", "points": [P(0, -3), P(10, -3)], "mark": "A",
         "target": {"view": "kesit-a"}, "broken": "targetGone"},
        {"kind": "stair", **base(26, "mimari"), "pts": [P(40, 0), P(43.75, 0)], "width": 1.2, "risers": 16,
         "riserHeight": 0.175, "tread": 0.25, "arch": {"level": "zemin"}},
        {"kind": "ramp", **base(27, "mimari"), "pts": [P(40, 5), P(50, 5)], "width": 1.5, "rise": 0.5, "slope": 5.0,
         "arch": {"level": "kat-9"}},
        # A room separator on no level (ADR 0717 §3): Denetim says it as a wall.
        {"kind": "polyline", **base(29, "mimari"), "lineType": "dashed", "pts": [P(52, 0), P(52, 4)],
         "spaceSeparator": {}},
        # Grid labels given twice (ADR 0725 §6): A on 22, again on it (no column: deleted), beside it (renamed B),
        # on the locked layer (no fix) and on another level (none); 1 twice on the same line, a column on the second
        # (renamed past 2, which a grid has, to 3).
        {"kind": "grid", **base(30, "mimari"), "pts": [P(30, -2), P(30, 6)], "name": "A"},
        {"kind": "grid", **base(31, "mimari"), "pts": [P(32, -2), P(32, 6)], "name": "A"},
        {"kind": "grid", **base(32, "kilitli"), "pts": [P(34, -2), P(34, 6)], "name": "A"},
        {"kind": "grid", **base(33, "mimari"), "pts": [P(36, -2), P(36, 6)], "name": "A", "arch": {"level": "kat-1"}},
        {"kind": "grid", **base(34, "mimari"), "pts": [P(28, 7), P(38, 7)], "name": "1"},
        {"kind": "grid", **base(35, "mimari"), "pts": [P(38, 7), P(28, 7)], "name": "1"},
        {"kind": "grid", **base(36, "mimari"), "pts": [P(28, 9), P(38, 9)], "name": "2"},
        {"kind": "column", **base(37, "mimari"), "at": P(32, 7), "width": 0.4, "depth": 0.4,
         "grids": [U(31), U(35)], "place": "A-1", "arch": {"level": "zemin"}},
    ]
    ds = {"id": SET, "wall": GONE["wall"], "face": "left", "rows": ["overall"], "openings": "ends", "layerId": "olcu",
          "at": P(20, -2.5), "gapMm": 7.0, "height": 0.125, "look": {},
          "members": [{"uid": U(21), "row": "overall", "a": vertex(GONE["wall"], 0), "b": vertex(GONE["wall"], 1)}],
          "broken": "hostGone"}
    # Each object keeps the uid of its name (U(28) is the level-less wall); its id is its place in the file, as the
    # KCAD writer requires (scripts/fixtures/kcad_v2_reference.py), so the same setup also runs in Python.
    uids = [U(e["id"]) for e in ents]
    for i, e in enumerate(ents):
        e["id"] = i + 1
    return {
        "format": "kentos.document", "version": 2, "name": "Mimari denetim",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50.0, "workspace": "cad", "arch": {"levels": LEVELS}},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("duvar", "Duvar"), layer("mimari", "Mimari"), layer("olcu", "Ölçü"),
                   layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False)],
        "activeLayer": "duvar",
        "entities": ents,
        "uids": uids,
        "styles": {"items": [], "categories": []},
        "dimensionSets": [ds],
    }


# ── Geometry (exact fractions where the walls are concerned) ────────────────────────────────────────────────────


def fx(p):
    return (F(p["x"]), F(p["y"]))


def on_segment(p, a, b):
    if wj.cross(wj.sub(b, a), wj.sub(p, a)) != 0:
        return False
    return min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def inside(p, ring):
    """Strictly inside a closed ring (even–odd), p on no edge."""
    c = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if p[0] < x:
                c = not c
    return c


def ring_edges(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


class Body:
    """A wall's body: its left and right faces (the solver's raw offsets), an open wall's ring capped square."""

    def __init__(self, lines, closed):
        left, right = lines[0], lines[-1]
        if closed:
            self.rings = [left, right]
            self.segments = ring_edges(left) + ring_edges(right)
        else:
            ring = list(left) + list(reversed(right))
            self.rings = [ring]
            self.segments = ring_edges(ring)
        self.closed = closed

    def holds(self, p):
        if any(on_segment(p, a, b) for a, b in self.segments):
            return True
        if self.closed:
            return inside(p, self.rings[0]) != inside(p, self.rings[1])
        return inside(p, self.rings[0])


def ray_hit(p, d, a, b):
    """Where the ray p + t·d (t > 0) meets segment a-b: t, or None."""
    e = wj.sub(b, a)
    den = wj.cross(d, e)
    if den == 0:
        return None
    w = wj.sub(a, p)
    t = wj.cross(w, e) / den
    s = wj.cross(w, d) / den
    if t > 0 and 0 <= s <= 1:
        return t
    return None


def path_segments(w):
    pts = [fx(p) for p in w["pts"]]
    n = len(pts)
    count = n if w.get("closed") else n - 1
    return [(pts[k], pts[(k + 1) % n]) for k in range(count)]


def hypot(v):
    """|v|, exact when it is (the cases' directions are axis-aligned or Pythagorean), else a float."""
    try:
        return wj.exact_len(v)
    except Exception:
        return F(math.hypot(float(v[0]), float(v[1])))


def shoelace(pts):
    s = F(0)
    for i in range(len(pts)):
        a, b = pts[i], pts[(i + 1) % len(pts)]
        s += a[0] * b[1] - b[0] * a[1]
    return s / 2


def centroid(pts):
    a = shoelace(pts)
    if a == 0:
        return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))
    cx = cy = F(0)
    for i in range(len(pts)):
        p, q = pts[i], pts[(i + 1) % len(pts)]
        k = p[0] * q[1] - q[0] * p[1]
        cx += (p[0] + q[0]) * k
        cy += (p[1] + q[1]) * k
    return (cx / (6 * a), cy / (6 * a))


def xy(p):
    return {"x": float(p[0]), "y": float(p[1])}


# ── The rule ────────────────────────────────────────────────────────────────────────────────────────────────────


def locked_layers(d):
    return {l["id"] for l in d["layers"] if l.get("locked")}


def step(command, inp):
    return {"command": command, "input": inp}


def finding(check, code, message, repair, uids, at, fix=None):
    f = {"check": check, "code": code, "message": message, "repair": repair, "uids": uids, "at": xy(at)}
    if fix:
        f["fix"] = fix
    return f


def walls_findings(d, max_gap):
    walls = [e for e in d["entities"] if e["kind"] == "wall"]
    levels = []
    for w in walls:
        level = (w.get("arch") or {}).get("level")
        if level not in levels:
            levels.append(level)
    out = []
    for level in levels:
        out += level_walls_findings(d, [w for w in walls if (w.get("arch") or {}).get("level") == level], max_gap)
    return out


def level_walls_findings(d, walls, max_gap):
    """The findings of one level's walls (in the drawing's order)."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    solver = wj.Solver([wj.from_json(w) for w in walls])
    solver.solve()
    out = []
    for i in solver.issues:
        what, repair = wj.MESSAGES[i["code"]]
        f = i["fix"]
        w = walls[f["wall"]]
        fix = None
        if w["layerId"] not in locked:
            fix = {"label": f"{'Başlangıç' if f['end'] == 'start' else 'Bitiş'} ucunu {MODE_LABEL[f['mode']]} yap",
                   "step": step("cad.walls.edit", {"uids": [uid[w["id"]]],
                                                   "operation": {"kind": "joins", f["end"]: f["mode"]}})}
        out.append(finding("walls", i["code"], what, repair, [uid[walls[k]["id"]] for k in i["walls"]],
                           tuple(F(c) for c in i["at"]), fix))
    members = {(m[0], m[1]) for j in solver.member_joints for m in j["members"] if m[1] is not None}
    bodies = [Body(solver.lines[k], walls[k].get("closed", False)) for k in range(len(walls))]
    all_ends = []
    for k, w in enumerate(walls):
        if w.get("closed"):
            continue
        pts = [fx(p) for p in w["pts"]]
        all_ends.append((k, 0, pts[0]))
        all_ends.append((k, 1, pts[-1]))
    free = []
    for k, e, at in all_ends:
        w = walls[k]
        if w.get("joins", {}).get(["start", "end"][e], "auto") == "none":
            continue
        if (k, ["start", "end"][e]) in members:
            continue
        if any((k2, e2) != (k, e) and wj.length2(wj.sub(at, p2)) <= F(JOIN_TOLERANCE) ** 2 for k2, e2, p2 in all_ends):
            continue
        if any(j != k and bodies[j].holds(at) for j in range(len(walls))):
            continue
        pts = [fx(p) for p in w["pts"]]
        d_out = wj.sub(pts[0], pts[1]) if e == 0 else wj.sub(pts[-1], pts[-2])
        free.append((k, e, at, d_out))
    gapped = set()
    for k, e, at, d_out in free:
        best = None
        for j in range(len(walls)):
            if j == k:
                continue
            ts = [t for a, b in bodies[j].segments if (t := ray_hit(at, d_out, a, b)) is not None]
            if not ts:
                continue
            g = min(ts) * hypot(d_out)
            if best is None or g < best[0]:
                best = (g, j)
        if best is None or float(best[0]) > max_gap:
            continue
        g, j = best
        gapped.add((k, e))
        w, other = walls[k], walls[j]
        end = ["start", "end"][e]
        crosses = any(ray_hit(at, d_out, a, b) is not None for a, b in path_segments(other))
        fix = None
        if crosses and w["layerId"] not in locked:
            fix = {"label": f"{'Başlangıç' if e == 0 else 'Bitiş'} ucunu öbür duvara uzat",
                   "step": step("cad.walls.edit", {"uids": [uid[w["id"]]],
                                                   "operation": {"kind": "extend", "to": uid[other["id"]], "end": end}})}
        out.append(finding("walls", "arch_wall_end_gap",
                           f"Duvarın {'başlangıç' if e == 0 else 'bitiş'} ucuyla öbür duvar arasında {float(g):.3f} m boşluk var; birleşmiyor.",
                           "Ucu öbür duvara uzatın; mahal ancak duvarlar birleşince kapanır.",
                           [uid[w["id"]], uid[other["id"]]], at, fix))
    rest = [x for x in free if (x[0], x[1]) not in gapped]
    for a in range(len(rest)):
        for b in range(a + 1, len(rest)):
            (k1, e1, p1, _), (k2, e2, p2, _) = rest[a], rest[b]
            dist = math.sqrt(float(wj.length2(wj.sub(p1, p2))))
            if not (JOIN_TOLERANCE < dist <= max_gap):
                continue
            mid = ((p1[0] + p2[0]) / 2, (p1[1] + p2[1]) / 2)
            if k1 == k2:
                message = f"Duvarın iki ucu arasında {dist:.3f} m boşluk var; duvar kapanmıyor."
                uids = [uid[walls[k1]["id"]]]
            else:
                message = f"İki duvarın uçları arasında {dist:.3f} m boşluk var; birleşmiyor."
                uids = [uid[walls[k]["id"]] for k in sorted((k1, k2))]
            out.append(finding("walls", "arch_wall_ends_apart", message,
                               "Bir ucu tutamaçla öbür uca taşıyın (Uç kenetiyle); uçlar buluşunca duvarlar birleşir.",
                               uids, mid))
    return out


def opening_point(e):
    pts = [fx(p) for p in e["place"]]
    return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))


def space_point(e):
    return centroid([fx(p) for p in e["parts"][0]["outer"]["pts"]])


def space_label(e):
    words = " ".join(x for x in (e.get("number"), e.get("name")) if x)
    return words or "Adsız mahal"


def openings_findings(d):
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "opening":
            continue
        n = e.get("number")
        if e.get("broken"):
            what, repair = split(OPENING_WORDS[e["broken"]])
            if n:
                what = f"{what[:-1]} ({n})."
            out.append(finding("openings", "arch_opening_broken", what, repair, [uid[e["id"]]], opening_point(e)))
        if not e.get("style"):
            message = f"“{n}” açıklığının türü yok." if n else "Numarasız açıklığın türü yok."
            out.append(finding("openings", "tag_missing_type", message, "Öznitelikler'de açıklığın Tür'ünü seçin.",
                               [uid[e["id"]]], opening_point(e)))
    return out


def ring_length(pts):
    return sum(math.hypot(float(pts[(i + 1) % len(pts)][0] - pts[i][0]), float(pts[(i + 1) % len(pts)][1] - pts[i][1]))
               for i in range(len(pts)))


def spaces_findings(d, min_area, min_width):
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "space":
            continue
        label = space_label(e)
        at = space_point(e)
        if e.get("broken"):
            what, repair = split(SPACE_WORDS[e["broken"]])
            out.append(finding("spaces", "arch_space_broken", f"{label}: {what}", repair, [uid[e["id"]]], at))
        if e.get("void"):
            # A void (a shaft, ADR 0708) is small and narrow by design.
            continue
        area = F(0)
        perimeter = 0.0
        for part in e["parts"]:
            outer = [fx(p) for p in part["outer"]["pts"]]
            area += abs(shoelace(outer))
            perimeter += ring_length(outer)
            for h in part.get("holes", []):
                hole = [fx(p) for p in h["pts"]]
                area -= abs(shoelace(hole))
                perimeter += ring_length(hole)
        area = float(area)
        repair = "İki duvar arasında kalan bir boşluk mahal sayılmış olabilir; gereksizse mahalı silin."
        if area < min_area:
            out.append(finding("spaces", "arch_space_small", f"{label}: alanı {area:.3f} m²; çok küçük bir parça.",
                               repair, [uid[e["id"]]], at))
        elif perimeter > 0 and 2 * area / perimeter < min_width:
            out.append(finding("spaces", "arch_space_narrow",
                               f"{label}: ortalama eni {2 * area / perimeter:.3f} m; çok dar bir parça.",
                               repair, [uid[e["id"]]], at))
    return out


def numbers_findings(d):
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    groups = {}
    for e in d["entities"]:
        kind = {"opening": "opening", "space": "space"}.get(e["kind"])
        n = e.get("number") if kind else None
        if n:
            groups.setdefault((kind, n), []).append(e)
    out = []
    for (kind, n), es in groups.items():
        if len(es) > 1:
            first = es[0]
            at = opening_point(first) if kind == "opening" else space_point(first)
            what = "açıklıkta" if kind == "opening" else "mahalde"
            out.append(finding("numbers", "tag_duplicate_number", f"“{n}” numarası {len(es)} {what}.",
                               "Numaralardan birini değiştirin (Öznitelikler ya da Numarala).",
                               [uid[e["id"]] for e in es], at))
    return out


def tags_findings(d):
    locked = locked_layers(d)
    uids = set(d["uids"])
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "tag" or e["of"] in uids:
            continue
        text = e.get("text", "")
        message = f"“{text.replace(chr(10), ' ')}” etiketinin nesnesi çizimde yok." if text else "Etiketin nesnesi çizimde yok."
        fix = None
        if e["layerId"] not in locked:
            fix = {"label": "Etiketi sil", "step": step("cad.entities.delete", {"uids": [uid[e["id"]]]})}
        out.append(finding("tags", "tag_target_gone", message, "Nesneyi geri getirin ya da etiketi silin.",
                           [uid[e["id"]]], fx(e["at"]), fix))
    return out


def dimensions_findings(d):
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    by_uid = {u: e for u, e in zip(d["uids"], d["entities"])}
    out = []
    members = set()
    for s in d.get("dimensionSets", []):
        ms = [m["uid"] for m in s.get("members", []) if m["uid"] in by_uid]
        members.update(ms)
        if not s.get("broken"):
            continue
        at = fx(by_uid[ms[0]]["a"]) if ms else fx(s["at"])
        fix = None
        if s["layerId"] not in locked:
            fix = {"label": "Diziyi bağımsız ölçülere ayır",
                   "step": step("cad.dimensionsets.edit", {"id": s["id"], "operation": {"kind": "detach"}})}
        out.append(finding("dimensions", "arch_dimension_set_broken", SET_WORDS[s["broken"]],
                           "Duvarı ya da açıklığı düzeltin; dizi kendiliğinden güncellenir.", ms, at, fix))
    for e in d["entities"]:
        if e["kind"] != "dimension" or uid[e["id"]] in members:
            continue
        refs = e.get("refs") or {}
        broken = [k for k in ("a", "b", "c") if (refs.get(k) or {}).get("broken")]
        if not broken:
            continue
        fix = None
        if e["layerId"] not in locked:
            fix = {"label": "Kopan noktaları serbest bırak",
                   "step": step("cad.dimensions.reassociate",
                                {"uid": uid[e["id"]], "points": [{"point": k, "to": None} for k in broken]})}
        out.append(finding("dimensions", "arch_dimension_broken",
                           f"İlişkili ölçünün bağı koptu ({', '.join(broken)}); noktaları son yerlerinde duruyor.",
                           "Yeniden bağla ile noktaları bir nesneye bağlayın ya da serbest bırakın.",
                           [uid[e["id"]]], fx(e["a"]), fix))
    return out


def grids_findings(d):
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    grids = [e for e in d["entities"] if e["kind"] == "grid"]
    tied = {g for e in d["entities"] if e["kind"] == "column" for g in e.get("grids", [])}
    key = lambda e: ((e.get("arch") or {}).get("level"), e["name"])  # noqa: E731
    taken = {key(e) for e in grids}
    groups = {}
    for e in grids:
        groups.setdefault(key(e), []).append(e)
    bulge = lambda e: (e.get("bulges") or [0.0])[0]  # noqa: E731

    def near(p, q):
        return abs(p["x"] - q["x"]) <= 1e-9 and abs(p["y"] - q["y"]) <= 1e-9

    def lies_on(g, h):
        (a, b), (c, e) = g["pts"], h["pts"]
        return (near(a, c) and near(b, e) and abs(bulge(g) - bulge(h)) <= 1e-12) or \
            (near(a, e) and near(b, c) and abs(bulge(g) + bulge(h)) <= 1e-12)

    out = []
    for k, group in groups.items():
        first = group[0]
        for g in group[1:]:
            u = uid[g["id"]]
            fix = None
            if g["layerId"] not in locked:
                if lies_on(g, first) and u not in tied:
                    fix = {"label": "Üst üste binen aksı sil", "step": step("cad.entities.delete", {"uids": [u]})}
                else:
                    label = next_label(g["name"])
                    while (k[0], label) in taken:
                        label = next_label(label)
                    taken.add((k[0], label))
                    fix = {"label": f"Aksı “{label}” olarak adlandır",
                           "step": step("cad.arch.edit", {"uids": [u], "operation": {"kind": "set", "label": label}})}
            (a, b) = g["pts"]
            mid = ((F(a["x"]) + F(b["x"])) / 2, (F(a["y"]) + F(b["y"])) / 2)
            out.append(finding("grids", "arch_grid_label_duplicate", f"“{g['name']}” etiketi bu katta birden çok aksta.",
                               "Akslardan birini yeniden adlandırın (Öznitelikler ya da Aksları numarala); üst üste binen "
                               "fazla aksı silin.", [uid[first["id"]], u], mid, fix))
    return out


def columns_findings(d):
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    what, repair = split(COLUMN_WORDS)
    for e in d["entities"]:
        if e["kind"] != "column" or not e.get("broken"):
            continue
        fix = None
        if e["layerId"] not in locked:
            fix = {"label": "Kolonu akslardan ayır",
                   "step": step("cad.arch.edit", {"uids": [uid[e["id"]]], "operation": {"kind": "detach"}})}
        out.append(finding("columns", "arch_column_grid_gone", what, repair, [uid[e["id"]]], fx(e["at"]), fix))
    return out


def viewmarks_findings(d):
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    what, repair = split(VIEWMARK_WORDS)
    for e in d["entities"]:
        if e["kind"] != "viewmark" or not e.get("broken"):
            continue
        message = f"“{e['mark']}” işareti: {what}" if e.get("mark") else what
        fix = None
        if e["layerId"] not in locked:
            fix = {"label": "İşareti hedefinden ayır",
                   "step": step("cad.viewmarks.edit", {"uids": [uid[e["id"]]], "operation": {"kind": "detach"}})}
        out.append(finding("viewmarks", "arch_viewmark_target_gone", message, repair, [uid[e["id"]]],
                           fx(e["points"][0]), fix))
    return out


def own_level(e):
    if e["kind"] == "polyline":
        return (e.get("spaceSeparator") or {}).get("level")
    return (e.get("arch") or {}).get("level")


def fix_level(d, inp):
    """The level a level finding's fix gives (ADR 0717 §3): the input's, else the ground level (elevation 0, else
    the first); None without levels."""
    levels = (d["settings"].get("arch") or {}).get("levels") or []
    if inp.get("level") is not None:
        return next((l for l in levels if l["id"] == inp["level"]), None)
    return next((l for l in levels if l["elevation"] == 0.0), levels[0] if levels else None)


def levels_findings(d, inp):
    levels = [l["id"] for l in ((d["settings"].get("arch") or {}).get("levels") or [])]
    if not levels:
        return []
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    locked = {l["id"] for l in d["layers"] if l["locked"]}
    give = fix_level(d, inp)
    out = []
    for e in d["entities"]:
        if e["kind"] not in POSSESSIVE or (e["kind"] == "polyline" and "spaceSeparator" not in e):
            continue
        level = own_level(e)
        if level in levels:
            continue
        who = POSSESSIVE[e["kind"]]
        message = f"{who} katı “{level}” projede yok." if level else f"{who} katı yok; projede {len(levels)} kat var."
        if e["kind"] == "space":
            at = space_point(e)
        elif e["kind"] == "column":
            at = fx(e["at"])
        else:
            at = fx(e["pts"][0])
        fix = None
        if e["layerId"] not in locked:
            fix = {"label": f"Katını {give['name']} yap",
                   "step": step("cad.levels.assign", {"level": give["id"], "uids": [uid[e["id"]]]})}
        out.append(finding("levels", "arch_level_missing", message,
                           "Düzelt ya da Kata ver ile katını verin; Öznitelikler'de Kat (L) de seçilir.",
                           [uid[e["id"]]], at, fix))
    return out


def audit(d, inp):
    max_gap = inp.get("maxGap", DEFAULTS["maxGap"])
    min_area = inp.get("minArea", DEFAULTS["minArea"])
    min_width = inp.get("minWidth", DEFAULTS["minWidth"])
    asked = set(inp.get("checks") or GROUPS)
    found = []
    for g in GROUPS:
        if g not in asked:
            continue
        found += {
            "walls": lambda: walls_findings(d, max_gap),
            "openings": lambda: openings_findings(d),
            "spaces": lambda: spaces_findings(d, min_area, min_width),
            "numbers": lambda: numbers_findings(d),
            "tags": lambda: tags_findings(d),
            "dimensions": lambda: dimensions_findings(d),
            "grids": lambda: grids_findings(d),
            "columns": lambda: columns_findings(d),
            "viewmarks": lambda: viewmarks_findings(d),
            "levels": lambda: levels_findings(d, inp),
        }[g]()
    order = {g: i for i, g in enumerate(GROUPS)}
    code_order = {c: i for g in GROUPS for i, c in enumerate(CODES[g])}
    found.sort(key=lambda f: (order[f["check"]], code_order[f["code"]], f["at"]["x"], f["at"]["y"]))
    return {"findings": found, "revision": "$current"}


def refused(inp, d=None):
    for key, what in (("maxGap", "Boşluk eşiği"), ("minArea", "Küçük mahal eşiği"), ("minWidth", "Dar mahal eşiği")):
        v = inp.get(key)
        if v is not None and v < 0:
            unit = "m²" if key == "minArea" else "m"
            return refusal("invalid_arch_audit", f"{what} {num(float(v))} {unit}; sıfır ya da daha büyük olmalı.", key)
    if d is not None and inp.get("level") is not None and fix_level(d, inp) is None:
        return refusal("level_not_found", f"Projede “{inp['level']}” kimlikli kat yok. Katı Proje ayarları › Mimari'de "
                                          f"ekleyin ya da var olan bir katı seçin.", "level")
    return None


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"canUndo": False, "dirty": False, "revision": "same"}
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}


def asked(d, inp, op="execute", expect=None):
    return {"op": op, "input": inp, "result": completed(audit(d, inp)), "within": 1e-9,
            "expect": dict(expect or SAME)}


def fixed(d, f):
    """The drawing after a finding's fix, as its command writes it (ADR 0703 §2's table)."""
    d = copy.deepcopy(d)
    ents = {u: e for u, e in zip(d["uids"], d["entities"])}
    cmd, inp = f["fix"]["step"]["command"], f["fix"]["step"]["input"]
    if cmd == "cad.walls.edit" and inp["operation"]["kind"] == "joins":
        w = ents[inp["uids"][0]]
        w.setdefault("joins", {}).update({k: v for k, v in inp["operation"].items() if k in ("start", "end")})
    elif cmd == "cad.walls.edit":
        w, other = ents[inp["uids"][0]], ents[inp["operation"]["to"]]
        pts = [fx(p) for p in w["pts"]]
        e = inp["operation"]["end"]
        at, d_out = (pts[0], wj.sub(pts[0], pts[1])) if e == "start" else (pts[-1], wj.sub(pts[-1], pts[-2]))
        t = min(t for a, b in path_segments(other) if (t := ray_hit(at, d_out, a, b)) is not None)
        new = wj.add(at, wj.mul(d_out, t))
        w["pts"][0 if e == "start" else -1] = xy(new)
    elif cmd == "cad.entities.delete":
        k = d["uids"].index(inp["uids"][0])
        del d["uids"][k]
        del d["entities"][k]
    elif cmd == "cad.dimensions.reassociate":
        e = ents[inp["uid"]]
        for p in inp["points"]:
            e["refs"].pop(p["point"])
    elif cmd == "cad.dimensionsets.edit":
        d["dimensionSets"] = [s for s in d["dimensionSets"] if s["id"] != inp["id"]]
    elif cmd == "cad.arch.edit" and inp["operation"]["kind"] == "set":
        # A grid named again: a column tied to it takes its place from the new label (ADR 0343 §3).
        e = ents[inp["uids"][0]]
        e["name"] = inp["operation"]["label"]
        for c in d["entities"]:
            if c["kind"] == "column" and inp["uids"][0] in c.get("grids", []):
                g = [ents[x] for x in c["grids"]]
                c["place"] = acc_place(g[0]["name"], g[1]["name"])
    elif cmd == "cad.arch.edit":
        e = ents[inp["uids"][0]]
        e.pop("grids", None)
        e.pop("broken", None)
    elif cmd == "cad.viewmarks.edit":
        e = ents[inp["uids"][0]]
        e.pop("target", None)
        e.pop("broken", None)
    elif cmd == "cad.levels.assign":
        e = ents[inp["uids"][0]]
        if e["kind"] == "polyline":
            e["spaceSeparator"] = dict(e["spaceSeparator"], level=inp["level"])
        else:
            e["arch"] = dict(e.get("arch") or {}, level=inp["level"])
    return d


def cases():
    d = drawing()
    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    case("Bütün denetimler, varsayılan eşikler: her grubun bulguları sırasıyla", [asked(d, {})])
    case("Plan aynı cevabı verir, çizim değişmez", [asked(d, {}, op="plan")])
    case("Doğrulama: girdi geçerli", [{"op": "validate", "input": {}, "result": completed(None), "expect": dict(SAME)}])
    for g in GROUPS:
        case(f"Yalnız {g}", [asked(d, {"checks": [g]})])
    case("Grupların sırası girdinin sırası değil", [asked(d, {"checks": ["levels", "walls", "tags"]})])
    case("Boş grup listesi: hepsi", [asked(d, {"checks": []})])
    case("Boşluk eşiği 3 cm: 4 cm'lik uç boşluğu ve 5 cm'lik uç aralığı bulgu değil",
         [asked(d, {"checks": ["walls"], "maxGap": 0.03})])
    case("Boşluk eşiği 4,5 cm: uç boşluğu bulgu, 5 cm'lik aralık değil", [asked(d, {"checks": ["walls"], "maxGap": 0.045})])
    case("Boşluk eşiği sıfır: boşluk bulgusu yok", [asked(d, {"checks": ["walls"], "maxGap": 0.0})])
    case("Küçük mahal eşiği 0,3 m²: şerit artık küçük sayılır", [asked(d, {"checks": ["spaces"], "minArea": 0.3})])
    case("Dar mahal eşiği sıfır: dar bulgusu yok", [asked(d, {"checks": ["spaces"], "minWidth": 0.0})])
    case("Mahal eşikleri sıfır: yalnız kopuk mahal", [asked(d, {"checks": ["spaces"], "minArea": 0.0, "minWidth": 0.0})])
    for key in ("maxGap", "minArea", "minWidth"):
        inp = {key: -0.01}
        case(f"Eksi {key} reddedilir", [{"op": "execute", "input": inp, "result": refused(inp), "expect": dict(SAME)}])
    for key, what in (("maxGap", "Boşluk eşiği"), ("minArea", "Küçük mahal eşiği"), ("minWidth", "Dar mahal eşiği")):
        case(f"Sonlu olmayan {key} reddedilir", [{
            "op": "execute", "input": {key: 0.1}, "nonFinite": {key: "NaN"},
            "result": refusal("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", key),
            "expect": dict(SAME)}])

    # ADR 0717 §3: the level fixes give the input's level; one the project lacks is refused.
    case("Kat düzeltmesi girdinin katını verir: 1. kat", [asked(d, {"checks": ["levels"], "level": "kat-1"})])
    for name, inp in (("Projede olmayan düzeltme katı reddedilir", {"level": "kat-9"}),
                      ("Boş düzeltme katı reddedilir", {"checks": ["levels"], "level": ""})):
        case(name, [{"op": "execute", "input": inp, "result": refused(inp, d), "expect": dict(SAME)}])
    flat_level = copy.deepcopy(d)
    flat_level["settings"].pop("arch")
    case("Katsız projede düzeltme katı verilemez",
         [{"op": "execute", "input": {"level": "zemin"}, "result": refused({"level": "zemin"}, flat_level),
           "expect": dict(SAME)}], setup=flat_level)

    # Projede kat yoksa kat bulgusu yok.
    flat = copy.deepcopy(d)
    flat["settings"].pop("arch")
    case("Projede kat yok: kat denetimi bulgu vermez", [asked(flat, {"checks": ["levels"]})], setup=flat)

    # A shaft marked a void (ADR 0708): no small or narrow finding for it.
    voided = copy.deepcopy(d)
    for e in voided["entities"]:
        if e["kind"] == "space" and e.get("name") == "Şaft":
            e["void"] = True
    case("Boşluk işaretli şaft küçük ya da dar mahal sayılmaz", [asked(voided, {"checks": ["spaces"]})], setup=voided)

    # An empty drawing.
    empty = copy.deepcopy(d)
    # No recipes: the key goes too, as a writer omits an empty list (schema 113 holds only nonempty dimensionSets).
    empty["entities"], empty["uids"] = [], []
    empty.pop("dimensionSets")
    case("Boş çizim: bulgu yok", [asked(empty, {})], setup=empty)

    # The rename fix (ADR 0725 §6): the grid named the next free label, a column tied to it taking its place from it.
    renamed = next(f for f in audit(d, {"checks": ["grids"]})["findings"]
                   if f.get("fix", {}).get("step", {}).get("command") == "cad.arch.edit")
    st = renamed["fix"]["step"]
    case("Düzelt ve yeniden denetle: aksı sıradaki boş etiketle adlandır", [
        asked(d, {"checks": ["grids"]}),
        {"op": "execute", "command": st["command"], "input": st["input"], "result": ANY,
         "expect": {"canUndo": True, "revision": "changed"}},
        asked(fixed(d, renamed), {"checks": ["grids"]}, expect={"canUndo": True, "revision": "same"}),
    ])

    # Each fix by its own command: the finding goes, undo brings it back (ADR 0703 §3: düzelt, yeniden denetle, geri al).
    full = audit(d, {})["findings"]
    done = set()
    for f in full:
        if "fix" not in f or f["code"] in done:
            continue
        done.add(f["code"])
        g = f["check"]
        after = fixed(d, f)
        st = f["fix"]["step"]
        case(f"Düzelt ve yeniden denetle: {f['code']}", [
            asked(d, {"checks": [g]}),
            {"op": "execute", "command": st["command"], "input": st["input"], "result": ANY,
             "expect": {"canUndo": True, "revision": "changed"}},
            asked(after, {"checks": [g]}, expect={"canUndo": True, "revision": "same"}),
            {"op": "undo", "expect": {"canUndo": False, "canRedo": True}},
            asked(d, {"checks": [g]}, expect={"canUndo": False, "canRedo": True, "revision": "same"}),
        ])
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Mimari denetim: duvar birleşimleri, uç boşlukları, kopuk açıklık, mahal, ölçü, kolon ve işaret, "
                     "yinelenen numara, katsız nesne; eşikler, düzeltmeler ve retler",
            "note": "ADR 0703. Beklenenler scripts/fixtures/arch_audit_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
                    "olmadan; duvar birleşimleri ve yüzler scripts/fixtures/wall_join_cases.py'nin kesirli çözücüsünden.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
