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

ADR 0732 adds, by the same hand: an open wall shorter than `minLength` (0.05 m; `arch_wall_short`, fix delete unless it
holds an opening or is locked), two straight walls of a level along each other (`arch_wall_overlap`: their faces'
middles parallel within a degree, nearer across than their half widths together, overlapping along for more than the
wider; fix delete the later copy), two room separators of a level on one line (`arch_separator_overlap`), an empty tag
(`tag_empty`), the stairs' and the ramps' own warnings (`arch_command_cases.py`'s; `stair_rise_level` fixed by the
height from the level, `ramp_slope_mismatch` by the slope made), a broken kitchen run (`arch_casework_broken`, fix
release). Each finding's key (`code:uids`, an end gap's `:start`/`:end`, `#2` for a second alike), the part asked
(`within`), the project's exceptions (a digest of the canonical JSON of the finding's code, objects, words and place:
the first eight bytes of its SHA-256) and `cad.arch.exceptions`'s cases, written to `cad.arch.exceptions.json`.
"""

import argparse
import copy
import hashlib
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_join_cases as wj  # noqa: E402  (the joins' reference: exact fractions, straight walls)
import arch_command_cases as acmd  # noqa: E402  (the stairs' and the ramps' own warnings, ADR 0343)
from arch_command_cases import next_label  # noqa: E402  (the grid labels' rule, ADR 0343)
from protocol_cases import canonical  # noqa: E402  (canonical JSON, ADR 0253 §2)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.audit.json"
OUT_EXCEPTIONS = ROOT / "fixtures/commands/v1/cad.arch.exceptions.json"
COMMAND = "cad.arch.audit"
JOIN_TOLERANCE = 1e-3
DEFAULTS = {"maxGap": 0.10, "minArea": 0.10, "minWidth": 0.15, "minLength": 0.05}
GROUPS = ["walls", "openings", "spaces", "numbers", "tags", "dimensions", "grids", "columns", "viewmarks", "levels",
          "accessibility", "stairs", "ramps", "casework"]
RAMP_PROFILE = ["ramp_slope_over_profile", "ramp_long_run_slope", "ramp_landing_short", "ramp_turn_narrow"]
STAIR_PROFILE = ["stair_riser_over_profile", "stair_tread_short", "stair_step_over_profile", "stair_width_narrow",
                 "stair_landing_short", "stair_winder_narrow", "stair_railing_missing"]
CODES = {
    "walls": ["wall_join_overlap", "wall_join_crowded", "wall_join_acute", "wall_join_unresolved", "arch_wall_end_gap",
              "arch_wall_ends_apart", "arch_wall_short", "arch_wall_overlap", "arch_separator_overlap"],
    "openings": ["arch_opening_broken", "tag_missing_type"],
    "spaces": ["arch_space_broken", "arch_space_small", "arch_space_narrow"],
    "numbers": ["tag_duplicate_number"],
    "tags": ["tag_target_gone", "tag_empty"],
    "dimensions": ["arch_dimension_set_broken", "arch_dimension_broken"],
    "grids": ["arch_grid_label_duplicate"],
    "columns": ["arch_column_grid_gone"],
    "viewmarks": ["arch_viewmark_target_gone"],
    "levels": ["arch_level_missing"],
    "accessibility": RAMP_PROFILE,
    "stairs": ["stair_short", "stair_long", "stair_rise_level"] + STAIR_PROFILE,
    "ramps": ["ramp_no_run", "ramp_slope_mismatch"],
    "casework": ["arch_casework_broken"],
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


def U2(n):
    return f"0192f5a1-7320-7000-8000-{n:012x}"


def extras():
    """ADR 0732 §4's drawing (uids U2(id)): a wall and its copy, two walls along each other, a 3 cm wall, a 4 cm one on
    the locked layer, a corner and a run's continuation (no finding), a copy on another level; separators along each
    other, a reversed twin, a copy on another level; an opening without a number and its empty tag; stairs with too
    short and too long a walking line and one whose risers miss its level's height; a ramp without a sloped run and
    one whose slope its rise and run do not make."""
    wa = wall(1, [(0, 0), (6, 0)], 0.2)
    sep = lambda id, pts, level="zemin": {"kind": "polyline", **base(id, "mimari"), "lineType": "dashed",  # noqa: E731
                                          "pts": [P(*q) for q in pts], "spaceSeparator": {"level": level}}
    ents = [
        wa,
        wall(2, [(0, 0), (6, 0)], 0.2),
        wall(3, [(0, 3), (6, 3)], 0.2),
        wall(4, [(2, 3.05), (8, 3.05)], 0.2),
        wall(5, [(10, 0), (10.03, 0)], 0.1),
        {**wall(6, [(14, 0), (14.04, 0)], 0.1), "layerId": "kilitli"},
        wall(7, [(30, 0), (34, 0)], 0.2),
        wall(8, [(34, 0), (34, 4)], 0.2),
        wall(9, [(40, 0), (44, 0)], 0.2),
        wall(10, [(44, 0), (48, 0)], 0.2),
        wall(11, [(0, 0), (6, 0)], 0.2, level="kat-1"),
        sep(12, [(0, 10), (6, 10)]),
        sep(13, [(2, 10), (8, 10)]),
        sep(14, [(0, 12), (4, 12)]),
        sep(15, [(4, 12), (0, 12)]),
        sep(16, [(0, 14), (4, 14)], level="kat-1"),
        sep(17, [(0, 14), (4, 14)]),
        opening(18, wa, U2(1), 3.0, 0.9),
        # An empty text is the KCAD writer's default: left out, as a saved drawing has it.
        {"kind": "tag", **base(19, "mimari"), "of": U2(18), "at": P(3, -1)},
        {"kind": "stair", **base(20, "mimari"), "pts": [P(50, 0), P(52, 0)], "width": 1.2, "risers": 16,
         "riserHeight": 0.1875, "tread": 0.25, "arch": {"level": "zemin"}},
        {"kind": "stair", **base(21, "mimari"), "pts": [P(50, 5), P(55, 5)], "width": 1.2, "risers": 16,
         "riserHeight": 0.1875, "tread": 0.25, "arch": {"level": "kat-1"}},
        {"kind": "stair", **base(22, "mimari"), "pts": [P(50, 10), P(53.75, 10)], "width": 1.2, "risers": 16,
         "riserHeight": 0.175, "tread": 0.25, "arch": {"level": "zemin"}},
        {"kind": "ramp", **base(23, "mimari"), "pts": [P(60, 0), P(70, 0)], "width": 1.5, "rise": 0.5, "slope": 8.0,
         "arch": {"level": "zemin"}},
        {"kind": "ramp", **base(24, "mimari"), "pts": [P(60, 10), P(60.5, 10), P(60.5, 10.5)], "width": 1.5,
         "rise": 0.5, "arch": {"level": "zemin"}},
    ]
    uids = [U2(e["id"]) for e in ents]
    for i, e in enumerate(ents):
        e["id"] = i + 1
    return {
        "format": "kentos.document", "version": 2, "name": "Mimari denetim ekleri",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50.0, "workspace": "cad", "arch": {"levels": LEVELS}},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("duvar", "Duvar"), layer("mimari", "Mimari"), layer("olcu", "Ölçü"),
                   layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False)],
        "activeLayer": "duvar",
        "entities": ents,
        "uids": uids,
        "styles": {"items": [], "categories": []},
    }


def kitchen():
    """A kitchen run whose walls are gone (ADR 0727 §5): the KCAD sample's run, its walls taken out, broken."""
    d = json.loads((ROOT / "fixtures/kcad/v2/casework.json").read_text())
    walls = {u for u, e in zip(d["uids"], d["entities"]) if e["kind"] == "wall"}
    kept = [(u, e) for u, e in zip(d["uids"], d["entities"]) if u not in walls]
    d["uids"] = [u for u, _ in kept]
    d["entities"] = [e for _, e in kept]
    for i, e in enumerate(d["entities"]):
        e["id"] = i + 1
    d["casework"][0]["broken"] = "wallGone"
    return d


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
    f = {"key": f"{code}:{','.join(uids)}", "check": check, "code": code, "message": message, "repair": repair,
         "uids": uids, "at": xy(at)}
    if fix:
        f["fix"] = fix
    return f


def digest(f):
    """The digest an exception binds a finding by (ADR 0732 §3)."""
    text = canonical({"at": f["at"], "code": f["code"], "message": f["message"], "uids": f["uids"]})
    return hashlib.sha256(text.encode()).digest()[:8].hex()


def walls_findings(d, max_gap, min_length=DEFAULTS["minLength"]):
    walls = [e for e in d["entities"] if e["kind"] == "wall"]
    levels = []
    for w in walls:
        level = (w.get("arch") or {}).get("level")
        if level not in levels:
            levels.append(level)
    out = []
    for level in levels:
        out += level_walls_findings(d, [w for w in walls if (w.get("arch") or {}).get("level") == level], max_gap,
                                    min_length)
    return out + separators_findings(d)


def level_walls_findings(d, walls, max_gap, min_length=DEFAULTS["minLength"]):
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
        f = finding("walls", "arch_wall_end_gap",
                    f"Duvarın {'başlangıç' if e == 0 else 'bitiş'} ucuyla öbür duvar arasında {float(g):.3f} m boşluk var; birleşmiyor.",
                    "Ucu öbür duvara uzatın; mahal ancak duvarlar birleşince kapanır.",
                    [uid[w["id"]], uid[other["id"]]], at, fix)
        f["key"] += ":" + end
        out.append(f)
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
    return out + short_walls(d, walls, min_length) + overlapping_walls(d, walls)


def hosts(d):
    return {e["host"] for e in d["entities"] if e["kind"] == "opening"}


def path_length(w):
    """A straight path's length (the cases' walls are straight)."""
    pts = [(p["x"], p["y"]) for p in w["pts"]]
    n = len(pts)
    count = n if w.get("closed") else n - 1
    return sum(math.hypot(pts[(k + 1) % n][0] - pts[k][0], pts[(k + 1) % n][1] - pts[k][1]) for k in range(count))


def short_walls(d, walls, min_length):
    """Open walls shorter than the least length (ADR 0732 §4)."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    held = hosts(d)
    out = []
    for w in walls:
        if w.get("closed") or len(w["pts"]) < 2:
            continue
        length = path_length(w)
        if length >= min_length:
            continue
        u = uid[w["id"]]
        a, b = w["pts"][0], w["pts"][-1]
        fix = None
        if w["layerId"] not in locked and u not in held:
            fix = {"label": "Kısa duvar parçasını sil", "step": step("cad.entities.delete", {"uids": [u]})}
        out.append(finding("walls", "arch_wall_short", f"Duvar {length:.3f} m; çok kısa bir parça.",
                           "Artık bir parçaysa silin; bilerek kısaysa istisna yapın.", [u],
                           ((a["x"] + b["x"]) / 2.0, (a["y"] + b["y"]) / 2.0), fix))
    return out


SIN_DEGREE = math.sin(math.radians(1.0))


def along_each_other(a, b):
    """Two straight runs along each other, as the core measures them in floats: how long they overlap along the
    first, the overlap's middle on it, how far apart across; None unless parallel within a degree."""
    dx, dy = a[1][0] - a[0][0], a[1][1] - a[0][1]
    ex, ey = b[1][0] - b[0][0], b[1][1] - b[0][1]
    la, lb = math.hypot(dx, dy), math.hypot(ex, ey)
    if la <= 1e-12 or lb <= 1e-12 or abs(dx * ey - dy * ex) > SIN_DEGREE * la * lb:
        return None
    ux, uy = dx / la, dy / la
    along = lambda p: (p[0] - a[0][0]) * ux + (p[1] - a[0][1]) * uy  # noqa: E731
    across = lambda p: abs((p[1] - a[0][1]) * ux - (p[0] - a[0][0]) * uy)  # noqa: E731
    t0, t1 = along(b[0]), along(b[1])
    lo, hi = max(min(t0, t1), 0.0), min(max(t0, t1), la)
    mid = (lo + hi) / 2.0
    return hi - lo, (a[0][0] + ux * mid, a[0][1] + uy * mid), (across(b[0]) + across(b[1])) / 2.0


def center_runs(w):
    """A straight wall's body's middle line, run by run: its raw faces' middles (the core's `wall::lines`)."""
    ww = wj.from_json(w)
    lines = [wj.offset_path(ww.pts, ww.closed, dd) for dd in ww.boundaries()]
    left, right = lines[0], lines[-1]
    mid = [((float(l[0]) + float(r[0])) / 2.0, (float(l[1]) + float(r[1])) / 2.0) for l, r in zip(left, right)]
    runs = [(mid[k], mid[k + 1]) for k in range(len(mid) - 1)]
    if w.get("closed") and len(mid) > 2:
        runs.append((mid[-1], mid[0]))
    return runs


def same_wall(a, b):
    if not (a["width"] == b["width"] and bool(a.get("closed")) == bool(b.get("closed")) and len(a["pts"]) == len(b["pts"])):
        return False
    forward = a.get("justify") == b.get("justify") and a.get("offset", 0.0) == b.get("offset", 0.0) and a["pts"] == b["pts"]
    centred = lambda w: w.get("justify", "center") == "center" and w.get("offset", 0.0) == 0.0  # noqa: E731
    backward = centred(a) and centred(b) and a["pts"] == list(reversed(b["pts"]))
    return forward or backward


def overlapping_walls(d, walls):
    """Straight walls of one level along each other (ADR 0732 §4), once a pair where they overlap longest."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    held = hosts(d)
    runs = [center_runs(w) for w in walls]
    out = []
    for i in range(len(walls)):
        for j in range(i + 1, len(walls)):
            ta, tb = walls[i]["width"], walls[j]["width"]
            best = None
            for a in runs[i]:
                for b in runs[j]:
                    r = along_each_other(a, b)
                    if r and r[0] > max(ta, tb) + JOIN_TOLERANCE and r[2] < (ta + tb) / 2.0 - JOIN_TOLERANCE:
                        if best is None or r[0] > best[0]:
                            best = r
            if best is None:
                continue
            length, at, _ = best
            ua, ub = uid[walls[i]["id"]], uid[walls[j]["id"]]
            fix = None
            if same_wall(walls[i], walls[j]):
                for k, u in ((j, ub), (i, ua)):
                    if walls[k]["layerId"] not in locked and u not in held:
                        fix = {"label": "Yinelenen duvarı sil", "step": step("cad.entities.delete", {"uids": [u]})}
                        break
            out.append(finding("walls", "arch_wall_overlap", f"İki duvar {length:.3f} m boyunca üst üste çizilmiş.",
                               "Fazla duvarı silin ya da kısaltın; mahal sınırında biri yok sayılabilir.", [ua, ub], at,
                               fix))
    return out


def separators_findings(d):
    """Room separators of one level on one line (ADR 0732 §4)."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    seps = [e for e in d["entities"] if e["kind"] == "polyline" and "spaceSeparator" in e]

    def edges(p):
        pts = [(q["x"], q["y"]) for q in p["pts"]]
        n = len(pts)
        bulges = p.get("bulges") or []
        count = n if p.get("closed") and n > 2 else n - 1
        return [(pts[k], pts[(k + 1) % n]) for k in range(count) if (bulges[k] if k < len(bulges) else 0.0) == 0.0]

    level = lambda p: (p.get("spaceSeparator") or {}).get("level")  # noqa: E731
    out = []
    for i in range(len(seps)):
        for j in range(i + 1, len(seps)):
            if level(seps[i]) != level(seps[j]):
                continue
            best = None
            for a in edges(seps[i]):
                for b in edges(seps[j]):
                    r = along_each_other(a, b)
                    if r and r[0] > 1e-3 and r[2] <= 1e-3 and (best is None or r[0] > best[0]):
                        best = r
            if best is None:
                continue
            length, at, _ = best
            ua, ub = uid[seps[i]["id"]], uid[seps[j]["id"]]
            twin = seps[i]["pts"] == seps[j]["pts"] or seps[i]["pts"] == list(reversed(seps[j]["pts"]))
            fix = None
            if twin and seps[j]["layerId"] not in locked:
                fix = {"label": "Yinelenen ayırıcıyı sil", "step": step("cad.entities.delete", {"uids": [ub]})}
            out.append(finding("walls", "arch_separator_overlap", f"İki mahal ayırıcı {length:.3f} m boyunca üst üste.",
                               "Fazla ayırıcıyı silin; mahal sınırını bir ayırıcı çizer.", [ua, ub], at, fix))
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
        if e["kind"] == "tag" and e["of"] in uids and not e.get("text", "").strip():
            out.append(finding("tags", "tag_empty", "Etiketin yazısı boş: nesnesinin şablondaki değerleri yok.",
                               "Nesnenin değerlerini verin (numara, ad, tür) ya da etiketin şablonunu değiştirin.",
                               [uid[e["id"]]], fx(e["at"])))
            continue
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


def level_height(d, level):
    for lv in (d["settings"].get("arch") or {}).get("levels") or []:
        if lv["id"] == level:
            return lv.get("height")
    return None


def stairs_findings(d):
    """The stairs' own checks (ADR 0732 §4): the drawing reference's warnings but Blondel's (information), the level's
    height by this drawing's levels."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "stair":
            continue
        u = uid[e["id"]]
        start = fx(e["pts"][0])
        for w in acmd.stair_warnings(e, "stair"):
            if w["code"] == "stair_short":
                what, repair = split(w["message"])
                out.append(finding("stairs", "stair_short", what, repair, [u], start))
            elif w["code"] == "stair_long":
                out.append(finding("stairs", "stair_long", w["message"],
                                   "Yolu son rıhtta bitirin (Merdiven'in Sığdır'ı) ya da rıht sayısını artırın.", [u], start))
        h = level_height(d, (e.get("arch") or {}).get("level"))
        if h is not None:
            rise = e["risers"] * e["riserHeight"]
            if abs(rise - h) > 1e-3:
                what = (f"{e['risers']} rıht × {acmd.fixed(e['riserHeight'], 3)} m = {acmd.fixed(rise, 3)} m; "
                        f"katın yüksekliği {acmd.fixed(h, 3)} m.")
                fix = None
                if e["layerId"] not in locked:
                    fix = {"label": "Yüksekliği kattan al",
                           "step": step("cad.arch.edit", {"uids": [u], "operation": {"kind": "set", "heightFromLevel": True}})}
                out.append(finding("stairs", "stair_rise_level", what,
                                   "Rıht yüksekliğini katın yüksekliğinden alın ya da rıht sayısını değiştirin.", [u], start,
                                   fix))
    return out


def ramp_made(r):
    """The slope a ramp's rise and sloped run make, per cent; None without a run."""
    pts = acmd.fpts(r["pts"])
    land = F(r["landing"]) if r.get("landing") is not None else F(r["width"])
    at = acmd.ac.walk(pts)
    run = at[-1] - sum(b - a for a, b in acmd.ac.ramp_landings(pts, at, land))
    return None if run <= F(1, 10**9) else r["rise"] / float(run) * 100


def ramps_findings(d):
    """The ramps' own checks (ADR 0732 §4)."""
    locked = locked_layers(d)
    uid = dict(zip([e["id"] for e in d["entities"]], d["uids"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "ramp":
            continue
        u = uid[e["id"]]
        start = fx(e["pts"][0])
        for w in acmd.ramp_warnings(e, "ramp"):
            if w["code"] == "ramp_no_run":
                out.append(finding("ramps", "ramp_no_run", w["message"], "Yolu uzatın ya da sahanlığı kısaltın.", [u], start))
            else:
                made = ramp_made(e)
                fix = None
                if made is not None and e["layerId"] not in locked:
                    fix = {"label": f"Eğimi %{acmd.fixed(made, 2)} yap",
                           "step": step("cad.arch.edit", {"uids": [u], "operation": {"kind": "set", "slope": made}})}
                out.append(finding("ramps", "ramp_slope_mismatch", w["message"],
                                   "Eğimi yüksekliğin eğimli uzunluğa oranına eşitleyin ya da yüksekliği veya yolu değiştirin.",
                                   [u], start, fix))
    return out


CASEWORK_WORDS = {
    "wallGone": "Mutfak dizisinin duvarı çizimde yok; dolaplar son yerlerinde duruyor.",
    "notBuilt": "Mutfak dizisi tarifinden kurulamıyor; dolaplar son yerlerinde duruyor.",
}


def casework_findings(d):
    """Kitchen runs that no longer follow (ADR 0732 §4); a run none of whose objects is left is not said."""
    locked = locked_layers(d)
    present = set(d["uids"])
    out = []
    for run in d.get("casework") or []:
        if not run.get("broken"):
            continue
        uids = [m["uid"] for m in run["members"] if m["uid"] in present]
        if not uids:
            continue
        fix = None
        if run["recipe"]["layerId"] not in locked:
            fix = {"label": "Diziyi bırak",
                   "step": step("cad.casework.edit", {"run": run["id"], "edit": {"kind": "release"}})}
        out.append(finding("casework", "arch_casework_broken", CASEWORK_WORDS[run["broken"]],
                           "Duvarı geri getirin ya da diziyi bırakın; bırakılan dizinin dolapları serbest nesneler olur.",
                           uids, fx(run["recipe"]["from"]), fix))
    return out


def entity_level(d, e):
    """An object's level as the level view reads it (ADR 0715 §1)."""
    by_uid = dict(zip(d["uids"], d["entities"]))

    def followed(x):
        if x is None:
            return None
        if x["kind"] == "opening":
            return own_level(by_uid[x["host"]]) if x["host"] in by_uid else None
        return own_level(x) if x["kind"] in ("wall", "space", "slab", "grid", "column", "stair", "ramp", "polyline") else None

    if e["kind"] == "opening":
        return followed(e)
    if e["kind"] == "tag":
        return followed(by_uid.get(e["of"]))
    if e["kind"] == "dimension":
        refs = e.get("refs") or {}
        level = None
        for k in ("a", "b", "c"):
            r = refs.get(k)
            if not r:
                continue
            lv = followed(by_uid.get(r["uid"]))
            if lv is None:
                return None
            if level is not None and lv != level:
                return None
            level = lv
        return level
    return followed(e)


def in_scope(d, f, within):
    if not within:
        return True
    by_uid = dict(zip(d["uids"], d["entities"]))
    levels = within.get("levels")
    if levels is not None and not any(entity_level(d, by_uid[u]) in levels for u in f["uids"] if u in by_uid):
        return False
    uids = within.get("uids")
    return uids is None or any(u in uids for u in f["uids"])


def found(d, inp):
    """Every finding of the groups asked, keyed, and the codes those groups could find (a profile's only when asked;
    these cases ask none)."""
    max_gap = inp.get("maxGap", DEFAULTS["maxGap"])
    min_area = inp.get("minArea", DEFAULTS["minArea"])
    min_width = inp.get("minWidth", DEFAULTS["minWidth"])
    min_length = inp.get("minLength", DEFAULTS["minLength"])
    asked = set(inp.get("checks") or GROUPS)
    out = []
    run = set()
    for g in GROUPS:
        if g not in asked:
            continue
        run |= {c for c in CODES[g] if c not in RAMP_PROFILE and c not in STAIR_PROFILE}
        out += {
            "walls": lambda: walls_findings(d, max_gap, min_length),
            "openings": lambda: openings_findings(d),
            "spaces": lambda: spaces_findings(d, min_area, min_width),
            "numbers": lambda: numbers_findings(d),
            "tags": lambda: tags_findings(d),
            "dimensions": lambda: dimensions_findings(d),
            "grids": lambda: grids_findings(d),
            "columns": lambda: columns_findings(d),
            "viewmarks": lambda: viewmarks_findings(d),
            "levels": lambda: levels_findings(d, inp),
            "accessibility": lambda: [],
            "stairs": lambda: stairs_findings(d),
            "ramps": lambda: ramps_findings(d),
            "casework": lambda: casework_findings(d),
        }[g]()
    order = {g: i for i, g in enumerate(GROUPS)}
    code_order = {c: i for g in GROUPS for i, c in enumerate(CODES[g])}
    out.sort(key=lambda f: (order[f["check"]], code_order[f["code"]], f["at"]["x"], f["at"]["y"]))
    seen = {}
    for f in out:
        seen[f["key"]] = seen.get(f["key"], 0) + 1
        if seen[f["key"]] > 1:
            f["key"] = f"{f['key']}#{seen[f['key']]}"
    return out, run


def answer(d, inp):
    """The output and the warnings (ADR 0732 §2, §3)."""
    every, run = found(d, inp)
    exceptions = (d["settings"].get("arch") or {}).get("auditExceptions") or []
    keys = {f["key"] for f in every}
    stale = [x["key"] for x in exceptions if x["code"] in run and x["key"] not in keys]
    view = inp.get("exceptions", "hide")
    outside = excepted = lapsed = 0
    listed = []
    for f in every:
        if not in_scope(d, f, inp.get("within")):
            outside += 1
            continue
        x = next((x for x in exceptions if x["key"] == f["key"]), None)
        if x is not None:
            if x.get("note") is not None:
                f["note"] = x["note"]
            if x["digest"] == digest(f):
                f["exception"] = True
                excepted += 1
            else:
                f["lapsed"] = True
                lapsed += 1
        marked = f.get("exception") or f.get("lapsed")
        if (view == "hide" and not f.get("exception")) or view == "show" or (view == "only" and marked):
            listed.append(f)
    out = {"findings": listed, "outside": outside, "excepted": excepted}
    if stale:
        out["stale"] = stale
    out["revision"] = "$current"
    warnings = []
    if lapsed:
        warnings.append({"code": "exception_lapsed",
                         "message": f"{lapsed} istisnanın bulgusu verildiğinden beri değişti; bulgular yeniden açık. "
                                    f"İstisnayı yeniden verin ya da kaldırın."})
    return out, warnings


def audit(d, inp):
    return answer(d, inp)[0]


def refused(inp, d=None):
    for key, what in (("maxGap", "Boşluk eşiği"), ("minArea", "Küçük mahal eşiği"), ("minWidth", "Dar mahal eşiği"),
                      ("minLength", "Kısa duvar eşiği")):
        v = inp.get(key)
        if v is not None and v < 0:
            unit = "m²" if key == "minArea" else "m"
            return refusal("invalid_arch_audit", f"{what} {num(float(v))} {unit}; sıfır ya da daha büyük olmalı.", key)
    if d is not None and inp.get("level") is not None and fix_level(d, inp) is None:
        return refusal("level_not_found", f"Projede “{inp['level']}” kimlikli kat yok. Katı Proje ayarları › Mimari'de "
                                          f"ekleyin ya da var olan bir katı seçin.", "level")
    within = inp.get("within") or {}
    levels = [lv["id"] for lv in ((d or {}).get("settings", {}).get("arch") or {}).get("levels") or []]
    for i, lv in enumerate(within.get("levels") or []):
        if lv is not None and lv not in levels:
            return refusal("level_not_found", f"Projede “{lv}” kimlikli kat yok. Katı Proje ayarları › Mimari'de "
                                              f"ekleyin ya da var olan bir katı seçin.", f"within.levels[{i}]")
    for i, u in enumerate(within.get("uids") or []):
        if u not in (d or {}).get("uids", []):
            return refusal("entity_not_found", f"“{u}” kimlikli nesne çizimde yok.", f"within.uids[{i}]")
    return None


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"canUndo": False, "dirty": False, "revision": "same"}
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}


def asked(d, inp, op="execute", expect=None, command=None):
    out, warnings = answer(d, inp)
    st = {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
          "within": 1e-9, "expect": dict(expect or SAME)}
    if command:
        st = {"op": op, "command": command, **{k: v for k, v in st.items() if k != "op"}}
    return st


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
    elif cmd == "cad.arch.edit" and "heightFromLevel" in inp["operation"]:
        # The stair's rise from its level (ADR 0724 §1): its risers kept, its riser height the level's over them.
        e = ents[inp["uids"][0]]
        e["heightFromLevel"] = True
        e["riserHeight"] = level_height(d, (e.get("arch") or {}).get("level")) / e["risers"]
    elif cmd == "cad.arch.edit" and "slope" in inp["operation"]:
        ents[inp["uids"][0]]["slope"] = inp["operation"]["slope"]
    elif cmd == "cad.casework.edit":
        d["casework"] = [r for r in d["casework"] if r["id"] != inp["run"]]
        if not d["casework"]:
            d.pop("casework")
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
    for key in ("maxGap", "minArea", "minWidth", "minLength"):
        inp = {key: -0.01}
        case(f"Eksi {key} reddedilir", [{"op": "execute", "input": inp, "result": refused(inp), "expect": dict(SAME)}])
    for key, what in (("maxGap", "Boşluk eşiği"), ("minArea", "Küçük mahal eşiği"), ("minWidth", "Dar mahal eşiği"),
                      ("minLength", "Kısa duvar eşiği")):
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
    out += new_checks_cases()
    out += scope_cases(d)
    out += exception_view_cases(d)
    return out


def fix_cases(name, x, group, f):
    """A finding's fix by its own command, the drawing checked again, undo (ADR 0703 §3)."""
    st = f["fix"]["step"]
    return {"name": name, "setup": x, "steps": [
        asked(x, {"checks": [group]}),
        {"op": "execute", "command": st["command"], "input": st["input"], "result": ANY,
         "expect": {"canUndo": True, "revision": "changed"}},
        asked(fixed(x, f), {"checks": [group]}, expect={"canUndo": True, "revision": "same"}),
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True}},
        asked(x, {"checks": [group]}, expect={"canUndo": False, "canRedo": True, "revision": "same"}),
    ]}


def new_checks_cases():
    """ADR 0732 §4: the new findings and their fixes."""
    x = extras()
    out = [{"name": "Yeni denetimler: kısa ve üst üste duvar, ayırıcı, boş etiket, merdiven ve rampanın kendi uyarıları",
            "setup": x, "steps": [asked(x, {})]}]
    for g in ("walls", "tags", "stairs", "ramps"):
        out.append({"name": f"Yeni denetimler, yalnız {g}", "setup": x, "steps": [asked(x, {"checks": [g]})]})
    out.append({"name": "Kısa duvar eşiği 2 cm: 3 cm'lik duvar bulgu değil", "setup": x,
                "steps": [asked(x, {"checks": ["walls"], "minLength": 0.02})]})
    out.append({"name": "Kısa duvar eşiği sıfır: kısa duvar bulgusu yok", "setup": x,
                "steps": [asked(x, {"checks": ["walls"], "minLength": 0.0})]})
    every = audit(x, {})["findings"]
    for code, name in (("arch_wall_short", "kısa duvar parçasını sil"), ("arch_wall_overlap", "yinelenen duvarı sil"),
                       ("arch_separator_overlap", "yinelenen ayırıcıyı sil"),
                       ("stair_rise_level", "merdivenin yüksekliğini kattan al"),
                       ("ramp_slope_mismatch", "rampanın eğimini hesaplanana eşitle")):
        f = next(f for f in every if f["code"] == code and "fix" in f)
        out.append(fix_cases(f"Düzelt ve yeniden denetle: {name}", x, f["check"], f))
    k = kitchen()
    out.append({"name": "Duvarı silinen mutfak dizisi", "setup": k, "steps": [asked(k, {"checks": ["casework"]})]})
    f = audit(k, {"checks": ["casework"]})["findings"][0]
    out.append(fix_cases("Düzelt ve yeniden denetle: mutfak dizisini bırak", k, "casework", f))
    return out


def scope_cases(d):
    """ADR 0732 §2: the part of the drawing reported on."""
    out = []
    for name, within in (("Kapsam: 1. kat", {"levels": ["kat-1"]}),
                         ("Kapsam: zemin kat", {"levels": ["zemin"]}),
                         ("Kapsam: katsız nesneler", {"levels": [None]}),
                         ("Kapsam: zemin kat ve katsızlar", {"levels": ["zemin", None]}),
                         ("Kapsam: seçili duvar", {"uids": [U(1)]}),
                         ("Kapsam: seçili iki nesne", {"uids": [U(2), U(26)]}),
                         ("Kapsam: zemin kattaki seçili duvar", {"levels": ["zemin"], "uids": [U(28)]}),
                         ("Kapsam: boş kat listesi hiçbir bulguyu tutmaz", {"levels": []})):
        out.append({"name": name, "steps": [asked(d, {"within": within})]})
    for name, within in (("Kapsamın olmayan katı reddedilir", {"levels": ["zemin", "kat-9"]}),
                         ("Kapsamın olmayan nesnesi reddedilir", {"uids": [U(0xdead)]})):
        inp = {"within": within}
        out.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": refused(inp, d), "expect": dict(SAME)}]})
    return out


def excepted_drawing(d):
    """The drawing with four exceptions: the gone tag's (as it is), the level-less wall's (its finding changed since:
    lapsed), a gone short wall's (stale), a stair profile's (not stale: no profile asked)."""
    x = copy.deepcopy(d)
    every = audit(d, {})["findings"]
    tag = next(f for f in every if f["code"] == "tag_target_gone" and f["uids"] == [U(19)])
    wall28 = next(f for f in every if f["code"] == "arch_level_missing" and f["uids"] == [U(28)])
    x["settings"]["arch"] = dict(x["settings"]["arch"], auditExceptions=[
        {"key": tag["key"], "code": tag["code"], "uids": tag["uids"], "digest": digest(tag), "note": "Bilerek bırakıldı."},
        {"key": wall28["key"], "code": wall28["code"], "uids": wall28["uids"], "digest": "0000000000000000"},
        {"key": "arch_wall_short:" + U(0xdead), "code": "arch_wall_short", "uids": [U(0xdead)], "digest": "1111111111111111"},
        {"key": "stair_riser_over_profile:" + U(26), "code": "stair_riser_over_profile", "uids": [U(26)],
         "digest": "2222222222222222"},
    ])
    return x


def exception_view_cases(d):
    """ADR 0732 §3: the project's exceptions laid on the findings."""
    x = excepted_drawing(d)
    out = []
    for name, inp in (("İstisnalar: varsayılan gizler, düşeni açar, bulgusu kalmayanı söyler", {}),
                      ("İstisnalar gösterilir", {"exceptions": "show"}),
                      ("Yalnız istisnalar", {"exceptions": "only"}),
                      ("İstisnalar gizli, yalnız etiketler: başka grubun istisnası bayat sayılmaz", {"checks": ["tags"]}),
                      ("İstisnalar ve kapsam: kapsam dışındaki istisna sayılmaz", {"within": {"levels": ["zemin"]}})):
        out.append({"name": name, "setup": x, "steps": [asked(x, inp)]})
    return out


def exceptions_cases():
    """`cad.arch.exceptions`' cases (ADR 0732 §3)."""
    d = drawing()
    every = audit(d, {})["findings"]
    tag = next(f for f in every if f["code"] == "tag_target_gone" and f["uids"] == [U(19)])
    level = next(f for f in every if f["code"] == "arch_level_missing" and f["uids"] == [U(28)])
    gap = next(f for f in every if f["code"] == "arch_wall_end_gap")

    def with_exceptions(base, items):
        x = copy.deepcopy(base)
        x["settings"]["arch"] = dict(x["settings"]["arch"], auditExceptions=items)
        return x

    def record(f, note=None):
        r = {"key": f["key"], "code": f["code"], "uids": f["uids"], "digest": digest(f)}
        if note is not None:
            r["note"] = note
        return r

    def run(inp, changed, count, expect=None):
        return {"op": "execute", "command": "cad.arch.exceptions", "input": inp,
                "result": completed({"changed": changed, "exceptions": count, "revision": "$current"}),
                "expect": dict(expect or {"canUndo": True, "revision": "changed", "dirty": True})}

    def refuse(inp, code, message, path, base=None):
        return {"op": "execute", "command": "cad.arch.exceptions", "input": inp,
                "result": refusal(code, message, path), "expect": dict(SAME)}

    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    one = with_exceptions(d, [record(tag, "Bilerek bırakıldı.")])
    case("İstisna yap: etiket bulgusu gizlenir, geri al döndürür", [
        run({"operation": "mark", "keys": [tag["key"]], "note": "Bilerek bırakıldı."}, [tag["key"]], 1),
        asked(one, {}, expect={"canUndo": True, "revision": "same"}, command="cad.arch.audit"),
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True}},
        asked(d, {}, expect={"canUndo": False, "canRedo": True, "revision": "same"}, command="cad.arch.audit"),
        {"op": "redo", "expect": {"canUndo": True, "canRedo": False}},
        asked(one, {}, expect={"canUndo": True, "canRedo": False, "revision": "same"}, command="cad.arch.audit"),
    ])
    two = with_exceptions(d, [record(tag), record(gap)])
    case("İki bulgu bir adımda istisna", [
        run({"operation": "mark", "keys": [tag["key"], gap["key"]]}, [tag["key"], gap["key"]], 2),
        asked(two, {"exceptions": "only"}, expect={"canUndo": True, "revision": "same"}, command="cad.arch.audit"),
    ])
    case("Zaten istisna olan bulgu değişmez, adım yazılmaz", [
        run({"operation": "mark", "keys": [tag["key"]]}, [], 1, expect=SAME),
    ], setup=with_exceptions(d, [record(tag)]))
    case("Yeni not istisnayı değiştirir", [
        run({"operation": "mark", "keys": [tag["key"]], "note": "Mimar onayladı."}, [tag["key"]], 1),
        asked(with_exceptions(d, [record(tag, "Mimar onayladı.")]), {"exceptions": "show", "checks": ["tags"]},
              expect={"canUndo": True, "revision": "same"}, command="cad.arch.audit"),
    ], setup=with_exceptions(d, [record(tag, "Bilerek bırakıldı.")]))
    lapsed = with_exceptions(d, [dict(record(level), digest="0000000000000000")])
    case("Düşen istisna yeniden verilince bulguya bağlanır", [
        asked(lapsed, {"checks": ["levels"]}, command="cad.arch.audit"),
        run({"operation": "mark", "keys": [level["key"]]}, [level["key"]], 1),
        asked(with_exceptions(d, [record(level)]), {"checks": ["levels"]}, expect={"canUndo": True, "revision": "same"},
              command="cad.arch.audit"),
    ], setup=lapsed)
    case("İstisnayı kaldır: bulgu yeniden görünür", [
        run({"operation": "unmark", "keys": [tag["key"]]}, [tag["key"]], 0),
        asked(d, {}, expect={"canUndo": True, "revision": "same"}, command="cad.arch.audit"),
        {"op": "undo", "expect": {"canUndo": False, "canRedo": True}},
        asked(one, {}, expect={"canUndo": False, "canRedo": True, "revision": "same"}, command="cad.arch.audit"),
    ], setup=one)
    stale = with_exceptions(d, [record(tag), {"key": "arch_wall_short:" + U(0xdead), "code": "arch_wall_short",
                                               "uids": [U(0xdead)], "digest": "1111111111111111"}])
    case("İstisnaları temizle: bulgusu kalmayan silinir", [
        run({"operation": "prune"}, ["arch_wall_short:" + U(0xdead)], 1),
        asked(one if False else with_exceptions(d, [record(tag)]), {"exceptions": "show"},
              expect={"canUndo": True, "revision": "same"}, command="cad.arch.audit"),
    ], setup=stale)
    case("Temizlenecek istisna yoksa adım yazılmaz", [
        run({"operation": "prune"}, [], 1, expect=SAME),
    ], setup=with_exceptions(d, [record(tag)]))
    case("Temizleme yalnız sorulan grupların istisnasına bakar", [
        run({"operation": "prune", "audit": {"checks": ["tags"]}}, [], 2, expect=SAME),
    ], setup=stale)
    case("Denetimin girdisiyle bulunan bulgu: yalnız etiketler", [
        run({"operation": "mark", "keys": [tag["key"]], "audit": {"checks": ["tags"]}}, [tag["key"]], 1),
    ])
    case("Plan aynı cevabı verir, çizim değişmez", [
        {"op": "plan", "command": "cad.arch.exceptions", "input": {"operation": "mark", "keys": [tag["key"]]},
         "result": completed({"changed": [tag["key"]], "exceptions": 1, "revision": "$current"}), "expect": dict(SAME)},
    ])
    case("Doğrulama: girdi geçerli", [
        {"op": "validate", "command": "cad.arch.exceptions", "input": {"operation": "unmark", "keys": [tag["key"]]},
         "result": completed(None), "expect": dict(SAME)},
    ], setup=one)
    case("Anahtarsız istisna reddedilir", [refuse({"operation": "mark"}, "invalid_input",
         "Bulgu anahtarı verilmedi; cad.arch.audit'in verdiği anahtarlardan en az birini verin.", "keys")])
    case("Anahtarsız kaldırma reddedilir", [refuse({"operation": "unmark", "keys": []}, "invalid_input",
         "Bulgu anahtarı verilmedi; cad.arch.audit'in verdiği anahtarlardan en az birini verin.", "keys")])
    case("Yinelenen anahtar reddedilir", [refuse({"operation": "mark", "keys": [tag["key"], tag["key"]]}, "invalid_input",
         f"“{tag['key']}” anahtarı birden çok kez verildi.", "keys[1]")])
    case("1000 karakterden uzun not reddedilir", [refuse({"operation": "mark", "keys": [tag["key"]], "note": "a" * 1001},
         "invalid_input", "Not en çok 1000 karakterdir, satır sonu dışında denetim karakteri olmaz.", "note")])
    case("Denetim karakterli not reddedilir", [refuse({"operation": "mark", "keys": [tag["key"]], "note": "a\tb"},
         "invalid_input", "Not en çok 1000 karakterdir, satır sonu dışında denetim karakteri olmaz.", "note")])
    case("Olmayan bulgu istisna yapılamaz", [refuse({"operation": "mark", "keys": [tag["key"], "arch_wall_short:yok"]},
         "finding_not_found", "Çizimin bugünkü denetiminde “arch_wall_short:yok” anahtarlı bulgu yok; önce "
         "cad.arch.audit ile denetleyin.", "keys[1]")])
    case("Olmayan istisna kaldırılamaz", [refuse({"operation": "unmark", "keys": [gap["key"]]}, "finding_not_found",
         f"“{gap['key']}” anahtarlı istisna yok.", "keys[0]")], setup=one)
    case("Denetimin reddi audit yoluyla", [refuse({"operation": "mark", "keys": [tag["key"]], "audit": {"maxGap": -1.0}},
         "invalid_arch_audit", "Boşluk eşiği -1 m; sıfır ya da daha büyük olmalı.", "audit.maxGap")])
    case("Denetimin olmayan düzeltme katı audit yoluyla reddedilir", [
        refuse({"operation": "prune", "audit": {"level": "kat-9"}}, "level_not_found",
               "Projede “kat-9” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.",
               "audit.level")])
    case("Beklenen sürüm tutmazsa çakışma", [
        {"op": "execute", "command": "cad.arch.exceptions",
         "input": {"operation": "mark", "keys": [tag["key"]], "expectedRevision": "999"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict",
                                                    "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                                    "path": "expectedRevision", "revision": "$current"}},
         "expect": {"revision": "same"}},
    ])
    case("Geçersiz beklenen sürüm reddedilir", [refuse(
        {"operation": "mark", "keys": [tag["key"]], "expectedRevision": "on iki"}, "invalid_revision",
        "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
        "komutun planından alın.", "expectedRevision")])
    step = refuse({"operation": "mark", "keys": [tag["key"]], "audit": {"maxGap": 0.1}}, "not_finite",
                  "Boşluk eşiği sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "audit.maxGap")
    step["nonFinite"] = {"audit.maxGap": "NaN"}
    case("Denetimin sonlu olmayan eşiği audit yoluyla reddedilir", [step])
    case("Profil bulgusunun istisnası profilsiz denetimde bayat sayılmaz", [
        asked(with_exceptions(d, [{"key": "stair_riser_over_profile:" + U(26), "code": "stair_riser_over_profile",
                                   "uids": [U(26)], "digest": "2222222222222222"}]), {"checks": ["stairs"]},
              command="cad.arch.audit"),
        run({"operation": "prune"}, [], 1, expect=SAME),
    ], setup=with_exceptions(d, [{"key": "stair_riser_over_profile:" + U(26), "code": "stair_riser_over_profile",
                                  "uids": [U(26)], "digest": "2222222222222222"}]))
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Mimari denetim: duvar birleşimleri, uç boşlukları, kopuk açıklık, mahal, ölçü, kolon ve işaret, "
                     "yinelenen numara, katsız nesne; eşikler, düzeltmeler ve retler",
            "note": "ADR 0703. Beklenenler scripts/fixtures/arch_audit_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
                    "olmadan; duvar birleşimleri ve yüzler scripts/fixtures/wall_join_cases.py'nin kesirli çözücüsünden.",
            "setup": drawing(), "cases": cases()}


def exceptions_document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.arch.exceptions", "commandVersion": 1,
            "title": "Denetim istisnası: yap, kaldır, temizle; düşen istisna, not, retler ve geri alma",
            "note": "ADR 0732 §3. Beklenenler scripts/fixtures/arch_audit_cases.py'nin ADR'den yazılmış kurallarından, KentOS "
                    "kodu olmadan; özet bulgunun kodu, nesneleri, iletisi ve yerinin kanonik JSON'unun (protocol_cases.py) "
                    "SHA-256'sının ilk sekiz baytı.",
            "setup": drawing(), "cases": exceptions_cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    failed = 0
    for path, doc in ((OUT, document()), (OUT_EXCEPTIONS, exceptions_document())):
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                failed = 1
            else:
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return failed


if __name__ == "__main__":
    sys.exit(main())
