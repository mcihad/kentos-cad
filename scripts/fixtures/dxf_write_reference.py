#!/usr/bin/env python3
"""Independent check of the DXF writer's blocks (docs/adr/0144 §5), texts (docs/adr/0145 §7), leaders (docs/adr/0146 §8),
multi-part lines and points (docs/adr/0174 §5), multi-line texts (docs/adr/0182 §5) and text and dimension styles
(docs/adr/0183 §7).

Reads `fixtures/formats/v1/dxf-write/<name>.input.json` (the writer's input,
written by hand) and `<name>.dxf` (what the writer made of it, committed) and
checks the file against the input from the rules alone, with the standard
library and no KentOS code:

- the blocks the objects place, and those nested in them, are BLOCKs with a
  BLOCK_RECORD each, in the order of a depth-first walk from the drawing's
  inserts (a block after the ones it holds); an unused block is not written;
  each dimension's own anonymous block (*D1, *D2 …) comes before the block
  holding the dimension, the drawing's dimensions' after all of them;
- a name keeps its letters, a character DXF refuses becomes "_", a name
  DXF's case-blind comparison takes for another's gets " (2)";
- a BLOCK and everything in it belong to its record (group 330); its base
  point and one-line description are the definition's;
- a definition's object is on its layer's DXF name, or on 0 when it has none
  of its own; BYBLOCK (62 0, 370 -2) where it has no colour or weight;
- an INSERT names its block, sits at the insert's point, scales X and Z by
  the scale and Y by it or its negative (mirrored), turns by the rotation in
  degrees; KentOS's data carries the exact radians exactly when the degrees
  do not give them back; an insert of a block the input does not have is
  not written;
- a block with attribute definitions (docs/adr/0144 §7) is flagged so (70 2)
  and has an ATTDEF for each after its objects, on 0: its place, height,
  default, turn (none when 0), prompt and tag (white space and control
  characters "_", a tag taken before "_2"); an insert of it says attributes
  follow (66 1) and is followed by an ATTRIB for each, owned by it, on its
  layer: the text it shows (its value, else the default, else nothing),
  placed as the insert places the definition (height times the scale; the
  turn as a text's under the insert's similarity, a mirrored one turned a
  half turn more to stay readable, within 1e-9), then a SEQEND of its own;
- a text (a TEXT, an ATTRIB, an ATTDEF; docs/adr/0145 §7) without an
  alignment starts at its point (10) and has no 11, 72 or 73; an aligned one
  stands on its alignment point (11) exactly (an ATTRIB's where its insert
  places it, within 1e-9), its 72 and 73 (an attribute's 74) name its
  alignment (none written for 0), 72 and 11 before the second subclass
  marker, the vertical after it; its start (10) is its alignment's share of
  its height under 11 across its baseline (within 1e-8 m) and its share of
  its width back along it, the width every text of the same words, height
  and width factor implies being one (within 1e-8 m) and an average letter
  of 0.3 to 0.9 of its height; its width factor is 41 (none for 1); a
  masked TEXT has KentOS's "mask" item, no other has;
- a leader (docs/adr/0146 §8) is a LEADER of the dimension style Standard:
  its arrowhead drawn (71 1) unless it has none, straight (72 0); with a
  note made with an MTEXT (73 0) and a hookline (75 1): its vertices, then
  the landing's end, 2 heights from the last vertex along the note's
  direction on the side the last segment goes (its projection on that
  direction 0 or more: along it, 74 1; else against it, 74 0); without one
  made with nothing (73 3, 75 0), its vertices alone (76 their count); its
  height (40); the note's direction (211, its turn's cosine and sine, within
  1e-12). With a note the next entity is its MTEXT: the LEADER names it
  (340), it names the LEADER first among its owners (its reactor), on the
  same layer; it stands 2.5 heights past the last vertex on that side
  (within 1e-8 m), the middle of its left there (71 4) or of its right (71
  6), left to right (72 1), as high as the leader, in MTEXT's notation
  (control characters spaces, a backslash, brace or caret escaped), in the
  note's direction (11), over the drawing's background exactly when masked
  (90 3). KentOS's data names an open or dot arrowhead ("arrow"), no other;
  it holds the note exactly when MTEXT cannot (a control character in it),
  and the turn, when it does, exactly;
- a dimension of docs/adr/0147 §8 (Koordinat, Yay uzunluğu, Kırıklı
  yarıçap, Semt, Eğim, and an aligned one with Zemin): an ordinate is a
  DIMENSION of type 6 (70: 6 + 32, + 64 when it gives the east, its angle 0)
  measured from (0, 0) (10), its point (13) and its line's end (14); an arc
  length an ARC_DIMENSION of type 5 (70 37): the arc's ends (13, 14) and
  centre (15) exactly, 10 on the dimension arc (its radius the arc's plus
  the offset) halfway round it, 40 and 41 the ends' angles in [0, 2π), not
  partial (70 0) and without a leader (71 0); a jogged radius a
  LARGE_RADIAL_DIMENSION of type 9 (70 41): the true centre (10), the
  centre shown (13), the point on the arc (15) exactly, the jog's middle
  (14) the offset (within the room the centre shown leaves) and half the
  jog along the radius from the centre shown, half its sideways step back
  (within 1e-8 m), 40 0 and the jog's 45° in its overrides (DSTYLE 50);
  Semt, Eğim and the aligned one are aligned DIMENSIONs (70 33) of their two
  points (13, 14). Each has KentOS's "dimension" item: its style, offset and
  height exactly, a slope's two elevations ("za", "zb") and no other's; a
  masked one has KentOS's "mask" item, DIMTFILL 1 in its overrides and its
  block's MTEXT over the background (90 3), no other has; its block's MTEXT
  says the value the input gives it;
- a local project's drawing (docs/adr/0165 §2; the input's `unit`, mm or
  cm) is written in its unit: $INSUNITS 4 or 5 (6 in metres), every
  coordinate and length of the input (points, vertices, radii, heights,
  offsets, elevations, hatch spacings, block bases and attribute
  definitions) a thousand or a hundred times its metres, and the checks
  above are made against the input so scaled; angles, turns, ratios and an
  insert's own scale stay;
- a multi-line text (docs/adr/0182 §5; one with a line break, a box, a line
  spacing or letter formats) is an MTEXT, in model space or its block: its
  alignment its attachment (71; a baseline's the top of its side, the point
  the box's top), as high as it (40), its box's width (41, 0 without one),
  left to right (72), its line spacing (44, at least: 73 1) only when it has
  one, its direction (11, within 1e-12) with the exact turn in KentOS's data
  only when the direction does not give it back, over the drawing's
  background exactly when masked (90 3); its words in MTEXT's notation read
  back by the notation's rules are its own, its runs the input's (a raised
  or lowered run whose letters a stack cannot hold on the line), its width
  factor a width switch before the first letter;
- text and dimension styles (docs/adr/0183 §7): the STYLE table holds
  Standard as it always was (Arial, no extended data), then each of the
  project's text styles under its name (DXF's refused characters "_", a
  name taken, case aside, " 2", " 3" …, Standard's taken first, the
  dimension styles sharing the names), its fixed height in drawing units
  (40; 0 without one), width (41) and slant (50), its typeface's file (3:
  its own, else Arial's and Courier's as AutoCAD names them, KentOS's
  others "‹Aile›.ttf"), ACAD's family and flags (34, bold 0x2000000, italic
  0x1000000) and the style itself in KentOS's data ("face"); then a record
  "KENTOS_‹AİLE›[_B][_I]" for each typeface a styleless text or a
  dimension's value has, in the families' order, KentOS's data saying
  "styleless". A TEXT names its record (7; none for Standard) and its own
  slant (51) when it has a typeface, an MTEXT names its record, both carry
  their face in KentOS's data exactly when they have one. The DIMSTYLE
  table holds Standard (no extended data) and each dimension style: its
  value's height (140), DIMSCALE 1, ticks as DIMTSZ (half a 45° tick's
  length) or arrowheads as DIMASZ (0 for none), the extension lines'
  offset (42) and reach (44), the value's gap (147) and place (77 0:
  centred), decimals (271), DIMPOST (3) after the flags, DIMLFAC (144, the
  value's unit in the file's) and the style in KentOS's data ("look"). A
  DIMENSION names its record (3) and overrides (DSTYLE) every one of these
  with its own look, carries its look in KentOS's data exactly when it has
  one; its block draws a filled arrow as a SOLID (tip on the dimension
  line's end, its base a size inwards, a third of the size wide), a dot as
  a DONUT (a closed two-vertex polyline, bulges 1, as wide as the dot's
  radius, a quarter of the size), an open arrow as two lines, and names its
  value's typeface's record (7);
- every handle is unique and every owner names a handle of the file.

    python3 scripts/fixtures/dxf_write_reference.py --check
"""

from __future__ import annotations

import json
import math
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "formats" / "v1" / "dxf-write"
REFUSED = set('<>/\\":;?*|=`\'')


class Bad(Exception):
    pass


def ensure(ok: bool, what: str) -> None:
    if not ok:
        raise Bad(what)


def pairs(data: bytes) -> list[tuple[int, str]]:
    lines = data.decode("utf-8").split("\r\n")
    ensure(lines[-1] == "", "the file ends with a line break")
    return [(int(lines[i].strip()), lines[i + 1]) for i in range(0, len(lines) - 1, 2)]


def section(p: list[tuple[int, str]], name: str) -> list[tuple[int, str]]:
    for i in range(len(p) - 1):
        if p[i] == (0, "SECTION") and p[i + 1] == (2, name):
            end = next(k for k in range(i, len(p)) if p[k] == (0, "ENDSEC"))
            return p[i + 2 : end]
    raise Bad(f"no {name} section")


def split(p: list[tuple[int, str]]) -> list[list[tuple[int, str]]]:
    """The entities (or records) of a run of groups, each from its group 0."""
    out: list[list[tuple[int, str]]] = []
    for g in p:
        if g[0] == 0:
            out.append([g])
        elif out:
            out[-1].append(g)
    return out


def group(e: list[tuple[int, str]], code: int) -> str | None:
    return next((v for c, v in e if c == code), None)


# A local project's unit: how many make a metre, and the $INSUNITS code naming it (docs/adr/0165 §2).
UNITS = {"mm": (1000.0, "4"), "cm": (100.0, "5"), "m": (1.0, "6")}
POINT_KEYS = {"p", "a", "b", "c", "base", "major"}
LENGTH_KEYS = {"r", "height", "offset", "z", "za", "zb"}


def scaled(o, k: float):
    """The input with every coordinate and length times `k`; angles, ratios and an insert's own scale as they are."""
    if isinstance(o, list):
        return [scaled(x, k) for x in o]
    if not isinstance(o, dict):
        return o
    xy = lambda q: {"x": q["x"] * k, "y": q["y"] * k}
    out = {}
    for key, v in o.items():
        if key in POINT_KEYS and isinstance(v, dict):
            out[key] = xy(v)
        elif key in ("pts", "ring"):
            out[key] = [xy(q) for q in v]
        elif key in LENGTH_KEYS and isinstance(v, (int, float)) and not isinstance(v, bool):
            out[key] = v * k
        elif key == "zs":
            out[key] = [None if z is None else z * k for z in v]
        elif key == "holes":
            # A polygon's rings ({pts, bulges, zs}) or a hatch's islands (lists of points).
            out[key] = [scaled(h, k) if isinstance(h, dict) else [xy(q) for q in h] for h in v]
        elif key == "pattern":
            out[key] = {**v, "spacing": v["spacing"] * k}
        elif key in ("entities", "blocks", "attributes", "parts"):
            out[key] = scaled(v, k)
        else:
            out[key] = v
    return out


def kentos(e: list[tuple[int, str]]) -> dict[str, list[str]]:
    """KentOS's items on an entity: tag -> the values after it (flat)."""
    items: dict[str, list[str]] = {}
    at = next((i for i, g in enumerate(e) if g == (1001, "KENTOS")), None)
    if at is None:
        return items
    i = at + 1
    while i < len(e):
        if e[i] == (1002, "{") and i + 1 < len(e) and e[i + 1][0] == 1000:
            tag = e[i + 1][1]
            values = []
            k = i + 2
            while k < len(e) and e[k] != (1002, "}"):
                values.append(e[k][1])
                k += 1
            items[tag] = values
            i = k + 1
        else:
            i += 1
    return items


def kentos_attrs(e: list[tuple[int, str]]) -> dict[str, str]:
    """KentOS's attributes on an entity (its "attr" items): key -> value."""
    out: dict[str, str] = {}
    at = next((i for i, g in enumerate(e) if g == (1001, "KENTOS")), None)
    if at is None:
        return out
    i = at + 1
    while i < len(e):
        if e[i] == (1002, "{") and i + 1 < len(e) and e[i + 1] == (1000, "attr"):
            k = i + 2
            values = []
            while k < len(e) and e[k] != (1002, "}"):
                values.append(e[k][1])
                k += 1
            ensure(len(values) == 2, f"an attribute is a key and a value: {values}")
            out[values[0]] = values[1]
            i = k + 1
        else:
            i += 1
    return out


def valid(name: str) -> str:
    s = "".join("_" if (ord(c) < 32 or c in REFUSED) else c for c in name.strip())[:255]
    return s or "Blok"


def dxf_names(order: list[dict]) -> dict[str, str]:
    taken: set[str] = set()
    out: dict[str, str] = {}
    for b in order:
        base = valid(b["name"]) if b["name"].strip() else "Blok"
        name, k = base, 2
        while name.upper() in taken:
            suffix = f" ({k})"
            name = base[: 255 - len(suffix)] + suffix
            k += 1
        taken.add(name.upper())
        out[b["id"]] = name
    return out


def walk(blocks: list[dict], entities: list[dict]) -> list[dict]:
    """Depth first from the drawing's inserts: a block after the ones it holds."""
    by_id = {b["id"]: b for b in blocks}
    done: set[str] = set()
    out: list[dict] = []

    def visit(i: str, open_: set[str]) -> None:
        if i in done or i in open_ or i not in by_id:
            return
        open_.add(i)
        for e in by_id[i]["entities"]:
            if e["kind"] == "insert":
                visit(e["block"], open_)
        open_.discard(i)
        done.add(i)
        out.append(by_id[i])

    for e in entities:
        if e["kind"] == "insert":
            visit(e["block"], set())
    return out


def turn_of(degrees: float) -> float:
    # The reader's rule (kentos_contracts::blocks::turn_of): into [0, 360), then radians.
    return ((math.fmod(degrees, 360.0) + 360.0) % 360.0) * math.pi / 180.0


# The DXF entities each object is written as, in order (a polygon's holes follow it).
def written_kinds(e: dict) -> list[str]:
    kind = e["kind"]
    # A table is its anonymous block's INSERT (docs/adr/0184 §7).
    if kind == "table":
        return ["INSERT"]
    if kind == "polygon":
        return ["LWPOLYLINE"] * (1 + len(e.get("holes") or []))
    if kind == "leader":
        return ["LEADER", "MTEXT"] if note_of(e) else ["LEADER"]
    if is_paragraph(e):
        return ["MTEXT"]
    if kind == "dimension":
        return [{"arcLength": "ARC_DIMENSION", "jogged": "LARGE_RADIAL_DIMENSION"}.get(e.get("style"), "DIMENSION")]
    # The fixture's blocks hold only these kinds.
    return [{"line": "LINE", "circle": "CIRCLE", "point": "POINT", "insert": "INSERT", "text": "TEXT"}[kind]]


def is_paragraph(e: dict) -> bool:
    """A text written as an MTEXT (docs/adr/0182 §5): it has a line break, a box, a line spacing or letter formats."""
    return e["kind"] == "text" and ("\n" in e["text"] or any(k in e for k in ("boxWidth", "lineSpacing", "runs")))


# MTEXT's attachment points (71) by KentOS's alignment; a baseline alignment goes out as the top's of its side.
ATTACH = {"topLeft": 1, "topCenter": 2, "topRight": 3, "middleLeft": 4, "middleCenter": 5, "middleRight": 6, "bottomLeft": 7, "bottomCenter": 8, "bottomRight": 9}
BASELINE_TOP = {None: 1, "baselineCenter": 2, "baselineRight": 3}
FORMAT_KEYS = ("bold", "italic", "underline", "script", "color")
UNSTACKABLE = set("^/#;\\{}%")


def read_mtext(s: str) -> tuple[str, list[dict], float | None]:
    """MTEXT content read by its notation's rules: its letters (\\P a line break; a backslash or brace escaped; %%% a
    per cent sign), the runs its groups and switches make (a font switch's b1 and i1, \\L and \\l, \\c a true colour
    with blue in the high byte, \\C7 the theme's ink; a stack over nothing raised, under nothing lowered) and a width
    switch before its first letter."""
    s = caret_decode(s)
    plain = {k: None for k in FORMAT_KEYS} | {"bold": False, "italic": False, "underline": False}
    cur, stack, letters, factor, i = dict(plain), [], [], None, 0
    while i < len(s):
        c = s[i]
        if c == "\\":
            k = s[i + 1]
            if k == "P" or k in "\\{}":
                letters.append(("\n" if k == "P" else k, dict(cur)))
                i += 2
                continue
            if k in "Ll":
                cur["underline"] = k == "L"
                i += 2
                continue
            end = s.index(";", i)
            body, i = s[i + 2 : end], end + 1
            if k == "W":
                ensure(not letters and not stack and factor is None, "a width switch only before the first letter")
                factor = float(body)
            elif k == "f":
                for part in body.split("|")[1:]:
                    if part[:1] in ("b", "i"):
                        cur["bold" if part[0] == "b" else "italic"] = part[1:2] == "1"
            elif k == "c":
                v = int(body)
                cur["color"] = "#%02X%02X%02X" % (v & 0xFF, (v >> 8) & 0xFF, v >> 16)
            elif k == "C":
                ensure(body == "7", f"\\C{body}: KentOS writes an ACI index only for the theme's ink")
                cur["color"] = "ink"
            elif k == "S":
                ensure("^" in body, f"a stack KentOS writes is a tolerance's: {body!r}")
                upper, lower = body.split("^", 1)
                raised = lower.strip() == ""
                words = upper if raised else lower.strip()
                letters.extend((x, dict(cur) | {"script": "super" if raised else "sub"}) for x in words)
            else:
                raise Bad(f"\\{k} is no switch KentOS writes")
            continue
        if c == "{":
            stack.append(dict(cur))
        elif c == "}":
            cur = stack.pop()
        elif s.startswith("%%%", i):
            letters.append(("%", dict(cur)))
            i += 3
            continue
        else:
            letters.append((c, dict(cur)))
        i += 1
    runs: list[dict] = []
    for n, (_, f) in enumerate(letters):
        if not (f["bold"] or f["italic"] or f["underline"] or f["script"] or f["color"]):
            continue
        if runs and runs[-1]["end"] == n and all(runs[-1].get(k) == (f[k] or None) for k in FORMAT_KEYS):
            runs[-1]["end"] = n + 1
        else:
            runs.append({"start": n, "end": n + 1, **{k: f[k] for k in FORMAT_KEYS if f[k]}})
    return "".join(x for x, _ in letters), runs, factor


def expected_runs(e: dict) -> list[dict]:
    """The input's runs as they come back: a raised or lowered run whose letters a stack cannot hold is on the line."""
    out = []
    for r in e.get("runs") or []:
        words = e["text"][r["start"] : r["end"]]
        if r.get("script") and (UNSTACKABLE & set(words) or any(control(x) for x in words)):
            r = {k: v for k, v in r.items() if k != "script"}
            if not any(k in r for k in FORMAT_KEYS):
                continue
        out.append(r)
    return out


def check_paragraph(o: list[tuple[int, str]], e: dict, where: str) -> None:
    """A multi-line text's MTEXT (docs/adr/0182 §5)."""
    r = math.radians(e["rotation"])
    h = e["height"]
    if e.get("align") in ATTACH:
        ensure(group(o, 71) == str(ATTACH[e["align"]]), f"{where}: attached as aligned")
        ensure(point(o, 10) == xy(e["p"]), f"{where}: at its point")
    else:
        ensure(group(o, 71) == str(BASELINE_TOP[e.get("align")]), f"{where}: a baseline's as the top of its side")
        if e.get("align") is None:
            # The box's top is a height over the first baseline.
            top = (e["p"]["x"] - math.sin(r) * h, e["p"]["y"] + math.cos(r) * h)
            got = point(o, 10)
            ensure(abs(got[0] - top[0]) <= 1e-9 and abs(got[1] - top[1]) <= 1e-9, f"{where}: at its box's top left {top}, not {got}")
    ensure(float(group(o, 40)) == h, f"{where}: its height")
    ensure(float(group(o, 41)) == float(e.get("boxWidth") or 0.0), f"{where}: its box's width (0 without one)")
    ensure(group(o, 72) == "1", f"{where}: left to right")
    if "lineSpacing" in e:
        ensure(group(o, 73) == "1" and float(group(o, 44)) == e["lineSpacing"], f"{where}: its line spacing")
    else:
        ensure(group(o, 44) is None, f"{where}: no line spacing of its own")
    d = point(o, 11)
    ensure(abs(d[0] - math.cos(r)) <= 1e-12 and abs(d[1] - math.sin(r)) <= 1e-12, f"{where}: its direction")
    turn = math.degrees(math.atan2(d[1], d[0])) % 360.0
    ensure(("noteturn" in kentos(o)) == (turn != e["rotation"]), f"{where}: its exact turn in KentOS's data only when the direction does not give it")
    ensure((group(o, 90) == "3") == bool(e.get("mask")), f"{where}: over the drawing's background exactly when masked")
    ensure("mask" not in kentos(o), f"{where}: the mask is the MTEXT's own")
    content = "".join(v for c, v in o if c == 3) + (group(o, 1) or "")
    text, runs, factor = read_mtext(content)
    want = "".join(" " if control(x) and x != "\n" else x for x in e["text"])
    ensure(text == want, f"{where}: its words {want!r}, not {text!r}")
    ensure(runs == expected_runs(e), f"{where}: its runs {expected_runs(e)}, not {runs}")
    ensure(factor == e.get("widthFactor"), f"{where}: its width factor")


def table_text(words: str) -> str:
    """A cell's words as a TEXT holds them: one line, the caret in DXF's notation, “%” tripled where “%%” would be a
    control code."""
    percent = "%%" in words
    out = []
    for c in words:
        if control(c):
            out.append(" ")
        elif c == "^":
            out.append("^ ")
        elif c == "%" and percent:
            out.append("%%%")
        else:
            out.append(c)
    return "".join(out)


TABLE_BASE = ("kind", "id", "layerId", "color", "attrs", "label", "symbol", "lineWeight")
TABLE_FLOATS = ("rotation", "height", "frame", "oblique")


def table_data(e: dict) -> str:
    """The table's own fields as KentOS's data holds them: the contract's JSON, keys in order, numbers as floats."""
    t = {k: v for k, v in e.items() if k not in TABLE_BASE}
    for k in TABLE_FLOATS:
        if k in t:
            t[k] = float(t[k])
    t["p"] = {"x": float(t["p"]["x"]), "y": float(t["p"]["y"])}
    t["rows"] = [float(x) for x in t["rows"]]
    t["columns"] = [float(x) for x in t["columns"]]
    return json.dumps(t, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def sums(sizes: list) -> list[float]:
    out = [0.0]
    for s in sizes:
        out.append(out[-1] + float(s))
    return out


def check_table(o: list[tuple[int, str]], e: dict, blocks: list, by_name: dict, layers: dict, where: str, n: int) -> None:
    """A table (docs/adr/0184 §7): an INSERT of the anonymous block *Un at its corner, turned as it is, KentOS's data
    its own fields; the block in its own axes (the corner at the origin, y down the page as −y): a LINE for every run of
    drawn edges (the outline's only without a frame band; none without lines), the band as four SOLIDs, a TEXT for every
    cell with words (a merged range's in its whole box), left, centred or right at half a height in, its baseline 0.35
    of a height under the middle, on 0 in BYBLOCK."""
    name = f"*U{n}"
    ensure(group(o, 2) == name and group(o, 330) == "17" and group(o, 8) == layers[e["layerId"]], f"{where}: {name}'s INSERT in model space, on its layer")
    ensure((float(group(o, 10)), float(group(o, 20))) == (float(e["p"]["x"]), float(e["p"]["y"])), f"{where}: at its corner")
    ensure((group(o, 50) is None) if e["rotation"] == 0 else float(group(o, 50)) == float(e["rotation"]), f"{where}: its turn")
    if e.get("color"):
        ensure(group(o, 420) == str(int(e["color"][1:], 16)), f"{where}: its true colour")
    data = kentos(o).get("table")
    ensure(data is not None and item_text(data) == table_data(e), f"{where}: KentOS's data is its fields")
    ensure(kentos_attrs(o) == e["attrs"], f"{where}: its attributes")
    k = by_name[name]
    end = next(i for i in range(k, len(blocks)) if blocks[i][0] == (0, "ENDBLK"))
    head, objects = blocks[k], blocks[k + 1 : end]
    ensure(group(head, 70) == "1" and (float(group(head, 10)), float(group(head, 20))) == (0.0, 0.0), f"{where}: anonymous, its base at the origin")
    ensure(all(group(x, 8) == "0" and group(x, 62) == "0" for x in objects), f"{where}: its block's objects on 0 in BYBLOCK")
    rows, columns, cells = e["rows"], e["columns"], e["cells"]
    nr, nc = len(rows), len(columns)
    xs, ys = sums(columns), sums(rows)
    merges = e.get("merges") or []
    grid = e.get("grid")
    band = e.get("frame") if grid != "none" else None

    def holds(g, i, j):
        return g["row"] <= i < g["row"] + g["rows"] and g["col"] <= j < g["col"] + g["cols"]

    def joined(a, b):
        return any(holds(g, *a) and holds(g, *b) for g in merges)

    def runs(drawn):
        out, start = [], None
        for j, d in enumerate(drawn + [False]):
            if d and start is None:
                start = j
            elif not d and start is not None:
                out.append((start, j))
                start = None
        return out

    lines = []
    if grid != "none":
        for r in range(nr + 1):
            outer = r in (0, nr)
            if (not outer and grid == "outer") or (outer and band is not None):
                continue
            for a, b in runs([outer or not joined((r - 1, j), (r, j)) for j in range(nc)]):
                lines.append(((xs[a], -ys[r]), (xs[b], -ys[r])))
        for c in range(nc + 1):
            outer = c in (0, nc)
            if (not outer and grid in ("outer", "rows")) or (outer and band is not None):
                continue
            for a, b in runs([outer or not joined((i, c - 1), (i, c)) for i in range(nr)]):
                lines.append(((xs[c], -ys[a]), (xs[c], -ys[b])))
    got = [((float(group(x, 10)), float(group(x, 20))), (float(group(x, 11)), float(group(x, 21)))) for x in objects if x[0] == (0, "LINE")]
    ensure(got == lines, f"{where}: its {len(lines)} lines")
    solids = [x for x in objects if x[0] == (0, "SOLID")]
    if band is None:
        ensure(not solids, f"{where}: no band")
    else:
        f, W, H = float(band), xs[-1], ys[-1]
        quads = [(0.0, 0.0, W, f), (0.0, H - f, W, H), (0.0, f, f, H - f), (W - f, f, W, H - f)]
        for (x0, y0, x1, y1), sx in zip(quads, solids):
            corners = [(float(group(sx, c)), float(group(sx, c + 10))) for c in (10, 11, 12, 13)]
            # SOLID runs 1 2 4 3: the strip's corners round it.
            ensure(corners == [(x0, -y0), (x1, -y0), (x0, -y1), (x1, -y1)], f"{where}: a band strip {corners}")
        ensure(len(solids) == 4, f"{where}: its band as four SOLIDs")
    texts = [x for x in objects if x[0] == (0, "TEXT")]
    h = float(e["height"])
    want = []
    for i, row in enumerate(cells):
        for j, words in enumerate(row):
            if not words.strip():
                continue
            span = next((g for g in merges if holds(g, i, j)), None)
            if span is not None and (span["row"], span["col"]) != (i, j):
                continue
            rr, cc = (span["rows"], span["cols"]) if span else (1, 1)
            x0, x1, y0, y1 = xs[j], xs[j + cc], ys[i], ys[i + rr]
            align = "center" if e.get("header") and i == 0 else (e.get("aligns") or ["left"] * nc)[j]
            x = {"left": x0 + 0.5 * h, "center": (x0 + x1) / 2.0, "right": x1 - 0.5 * h}[align]
            want.append((table_text(words), align, (x, -((y0 + y1) / 2.0 + 0.35 * h))))
    ensure(len(texts) == len(want), f"{where}: {len(want)} TEXTs")
    for t, (words, align, at) in zip(texts, want):
        ensure(group(t, 1) == words and float(group(t, 40)) == h, f"{where}: “{words}” at its height")
        if align == "left":
            ensure(group(t, 72) is None and (float(group(t, 10)), float(group(t, 20))) == at, f"{where}: “{words}” from its start")
        else:
            ensure(group(t, 72) == {"center": "1", "right": "2"}[align] and (float(group(t, 11)), float(group(t, 21))) == at, f"{where}: “{words}” {align}")


def note_of(e: dict) -> str | None:
    """A leader's note, when it has one that is not empty."""
    t = e.get("text")
    return t if t and t.strip() else None


def control(c: str) -> bool:
    return unicodedata.category(c) == "Cc"


def caret_decode(s: str) -> str:
    """DXF's caret notation back to characters ("^ " a caret, "^I" a tab)."""
    out, i = [], 0
    while i < len(s):
        if s[i] == "^" and i + 1 < len(s) and (s[i + 1] == " " or "@" <= s[i + 1] <= "_"):
            out.append("^" if s[i + 1] == " " else chr(ord(s[i + 1]) - 64))
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def mtext_notation(s: str) -> str:
    """A plain text in MTEXT's notation."""
    escape = {"\\": "\\\\", "{": "\\{", "}": "\\}", "^": "^ "}
    return "".join(" " if control(c) else escape.get(c, c) for c in s)


def check_leader(o: list[tuple[int, str]], m: list[tuple[int, str]] | None, e: dict, where: str) -> None:
    note = note_of(e)
    h = e["height"]
    r = math.radians(e["rotation"])
    u = (math.cos(r), math.sin(r))
    pts = [(p["x"], p["y"]) for p in e["pts"]]
    last = pts[-1]
    side = 1.0
    for q in reversed(pts[:-1]):
        d = (last[0] - q[0], last[1] - q[1])
        if d != (0.0, 0.0):
            side = 1.0 if d[0] * u[0] + d[1] * u[1] >= 0 else -1.0
            break
    along = lambda k: (last[0] + side * u[0] * k * h, last[1] + side * u[1] * k * h)
    ensure(o[0] == (0, "LEADER") and group(o, 3) == "Standard", f"{where}: a LEADER of Standard")
    ensure(group(o, 71) == ("0" if e.get("arrow") == "none" else "1"), f"{where}: its arrowhead drawn or not (71)")
    ensure(group(o, 72) == "0", f"{where}: straight (72 0)")
    ensure(group(o, 73) == ("0" if note else "3"), f"{where}: made with an MTEXT or with nothing (73)")
    ensure(group(o, 75) == ("1" if note else "0"), f"{where}: a hookline with a note (75)")
    if note:
        ensure(group(o, 74) == ("1" if side > 0 else "0"), f"{where}: the landing's side (74)")
    ensure(float(group(o, 40)) == h, f"{where}: its height")
    xs = [float(v) for c, v in o if c == 10]
    ys = [float(v) for c, v in o if c == 20]
    ensure(int(group(o, 76)) == len(xs) == len(pts) + (1 if note else 0), f"{where}: its vertices' count (76)")
    ensure(list(zip(xs, ys))[: len(pts)] == pts, f"{where}: its vertices exactly")
    if note:
        end = along(2.0)
        ensure(abs(xs[-1] - end[0]) <= 1e-8 and abs(ys[-1] - end[1]) <= 1e-8, f"{where}: the landing's end last ({end})")
    ensure(abs(float(group(o, 211)) - u[0]) <= 1e-12 and abs(float(group(o, 221)) - u[1]) <= 1e-12, f"{where}: the note's direction (211)")
    items = kentos(o)
    arrow = e.get("arrow")
    ensure(items.get("arrow") == ([arrow] if arrow in ("open", "dot") else None), f"{where}: KentOS's arrow item exactly for open and dot")
    if "noteturn" in items:
        ensure(float(items["noteturn"][0]) == e["rotation"], f"{where}: KentOS's turn is the leader's exactly")
    exact = bool(note) and any(control(c) for c in note)
    ensure(("note" in items) == exact, f"{where}: KentOS's note item exactly when MTEXT cannot say the note")
    if exact:
        ensure(caret_decode("".join(items["note"])) == note, f"{where}: KentOS's note exactly")
    if not note:
        ensure(group(o, 340) is None and m is None, f"{where}: no note, no MTEXT")
        return
    ensure(m is not None and m[0] == (0, "MTEXT"), f"{where}: its MTEXT next")
    ensure(group(o, 340) == group(m, 5), f"{where}: the LEADER names its MTEXT (340)")
    k = m.index((102, "{ACAD_REACTORS"))
    ensure(m[k + 1] == (330, group(o, 5)) and m[k + 2] == (102, "}"), f"{where}: the MTEXT's reactor names its LEADER")
    owners = [v for c, v in m if c == 330]
    ensure(owners == [group(o, 5), group(o, 330)], f"{where}: the MTEXT's owner is the LEADER's")
    ensure(group(m, 8) == group(o, 8), f"{where}: the MTEXT on the LEADER's layer")
    at = along(2.5)
    ensure(abs(float(group(m, 10)) - at[0]) <= 1e-8 and abs(float(group(m, 20)) - at[1]) <= 1e-8, f"{where}: the note 2.5 heights past the last vertex ({at})")
    ensure(float(group(m, 40)) == h, f"{where}: the note as high as the leader")
    ensure(group(m, 71) == ("4" if side > 0 else "6") and group(m, 72) == "1", f"{where}: the note's middle left or right (71), left to right")
    ensure(group(m, 1) == mtext_notation(note), f"{where}: the note in MTEXT's notation")
    ensure(abs(float(group(m, 11)) - u[0]) <= 1e-12 and abs(float(group(m, 21)) - u[1]) <= 1e-12, f"{where}: the note's direction (11)")
    ensure((group(m, 90) == "3") == bool(e.get("mask")), f"{where}: the background exactly when masked (90 3)")


def check_look(o: list[tuple[int, str]], e: dict, layers: dict[str, str], where: str) -> None:
    layer = layers.get(e["layerId"], "0")
    ensure(group(o, 8) == layer, f"{where}: on layer {layer!r}, not {group(o, 8)!r}")
    if e.get("color"):
        rgb = int(e["color"][1:], 16)
        ensure(group(o, 420) == str(rgb), f"{where}: its true colour {rgb}")
    else:
        ensure(group(o, 62) == "0", f"{where}: BYBLOCK colour (62 0)")
    if e.get("lineWeight") is None:
        ensure(group(o, 370) == "-2", f"{where}: BYBLOCK weight (370 -2)")


def dxf_tags(b: dict) -> list[str]:
    taken: set[str] = set()
    out = []
    for a in b.get("attributes") or []:
        base = "".join("_" if (c.isspace() or ord(c) < 32) else c for c in a["tag"])
        tag, k = base, 2
        while tag.upper() in taken:
            tag = f"{base}_{k}"
            k += 1
        taken.add(tag.upper())
        out.append(tag)
    return out


def close(a: float, b: float, tol: float = 1e-9) -> bool:
    return abs(a - b) <= tol * max(1.0, abs(b))


# KentOS's alignments as DXF's 72 and 73 (an attribute's 74): row by column.
JUSTIFY = {
    "baselineCenter": (1, 0), "baselineRight": (2, 0),
    "bottomLeft": (0, 1), "bottomCenter": (1, 1), "bottomRight": (2, 1),
    "middleLeft": (0, 2), "middleCenter": (1, 2), "middleRight": (2, 2),
    "topLeft": (0, 3), "topCenter": (1, 3), "topRight": (2, 3),
}
# Up from the baseline, in heights: the bottom is a fifth of a height under it.
ROWS = {"baseline": 0.0, "bottom": -0.2, "middle": 0.5, "top": 1.0}
COLUMNS = {"Left": 0.0, "Center": 0.5, "Right": 1.0}


def after(o: list[tuple[int, str]], marker: str) -> int:
    return next(k for k, g in enumerate(o) if g == (100, marker) and k > 0 and o[:k].count((100, marker)) == (1 if marker == "AcDbText" else 0))


def check_place(o: list[tuple[int, str]], p: tuple[float, float], exact: bool, t: dict, marker: str, vertical: int,
                shown: str, widths: dict, where: str) -> None:
    """A text's 10, 11, 72, 73 or 74 and 41 (`t`: its height, rotation, align, widthFactor)."""
    x10, y10 = float(group(o, 10)), float(group(o, 20))
    wf = t.get("widthFactor")
    ensure((group(o, 41) is None) if wf is None else float(group(o, 41)) == wf, f"{where}: width factor {wf}")
    split_at = after(o, marker)
    head, tail = o[:split_at], o[split_at:]
    align = t.get("align")
    if align is None:
        ensure(x10 == p[0] and y10 == p[1], f"{where}: starts at its point")
        ensure(all(c not in (11, 72) for c, _ in o) and group(tail, vertical) is None, f"{where}: not justified")
        return
    h, v = JUSTIFY[align]
    x11, y11 = float(group(o, 11)), float(group(o, 21))
    if exact:
        ensure(x11 == p[0] and y11 == p[1], f"{where}: stands on its point (11) exactly")
    else:
        ensure(close(x11, p[0]) and close(y11, p[1]), f"{where}: stands where its insert places it (11)")
    ensure(group(head, 11) is not None and group(tail, 11) is None, f"{where}: 11 before the {marker} marker")
    ensure((group(o, 72) is None) if h == 0 else (group(head, 72) == str(h)), f"{where}: 72 {h}")
    ensure(all(c != vertical for c, _ in head), f"{where}: {vertical} after the {marker} marker")
    ensure((group(tail, vertical) is None) if v == 0 else (group(tail, vertical) == str(v)), f"{where}: {vertical} {v}")
    row = next(r for r in ROWS if align.startswith(r))
    column = next(c for c in COLUMNS if align.endswith(c))
    a = math.radians(float(t["rotation"]))
    dx, dy = x11 - x10, y11 - y10
    across = -dx * math.sin(a) + dy * math.cos(a)
    run = dx * math.cos(a) + dy * math.sin(a)
    height = float(t["height"])
    ensure(abs(across - ROWS[row] * height) <= 1e-8, f"{where}: starts {ROWS[row]} of its height under 11, not {across / height}")
    if COLUMNS[column] == 0.0:
        ensure(abs(run) <= 1e-8, f"{where}: starts on 11's side")
    else:
        widths.setdefault((shown, height, wf or 1.0), []).append((run / COLUMNS[column], where))


def check_widths(widths: dict) -> None:
    for (shown, height, wf), found in widths.items():
        ws = [w for w, _ in found]
        ensure(max(ws) - min(ws) <= 1e-8, f"{shown!r}: one width, not {ws}")
        letter = ws[0] / (height * wf * len(shown))
        ensure(0.3 <= letter <= 0.9, f"{shown!r}: an average letter {letter} of its height")


def placed(e: dict, base: dict, q: dict) -> tuple[float, float]:
    flip = -1.0 if e.get("mirror") else 1.0
    s = float(e["scale"])
    x, y = s * (q["x"] - base["x"]), flip * s * (q["y"] - base["y"])
    c, n = math.cos(e["rotation"]), math.sin(e["rotation"])
    return e["p"]["x"] + c * x - n * y, e["p"]["y"] + n * x + c * y


def placed_turn(e: dict, degrees: float) -> float:
    # A text's direction under the insert's similarity; mirrored, a half turn more (readable).
    a = degrees * math.pi / 180.0
    flip = -1.0 if e.get("mirror") else 1.0
    dx, dy = math.cos(a), flip * math.sin(a)
    c, n = math.cos(e["rotation"]), math.sin(e["rotation"])
    turn = math.atan2(n * dx + c * dy, c * dx - n * dy) * 180.0 / math.pi
    if e.get("mirror"):
        turn += 180.0
    return ((turn % 360.0) + 360.0) % 360.0


def check_attributes(o: list[list[tuple[int, str]]], e: dict, b: dict, layer: str, own: str, where: str, widths: dict) -> None:
    """An insert's ATTRIBs and SEQEND (`o`: the entities after it; `own`: its handle)."""
    attributes = b.get("attributes") or []
    tags = dxf_tags(b)
    ensure(len(o) > len(attributes) and o[len(attributes)][0] == (0, "SEQEND"), f"{where}: {len(attributes)} ATTRIB, then SEQEND")
    ensure(group(o[len(attributes)], 330) == own, f"{where}: its SEQEND is its own")
    for k, (a, tag) in enumerate(zip(attributes, tags)):
        r = o[k]
        w = f"{where} › {a['tag']}"
        ensure(r[0] == (0, "ATTRIB"), f"{w}: an ATTRIB")
        ensure(group(r, 330) == own, f"{w}: owned by its insert")
        ensure(group(r, 8) == layer, f"{w}: on the insert's layer")
        ensure(group(r, 2) == tag, f"{w}: tag {tag!r}")
        value = (e.get("attrs") or {}).get(a["tag"]) or a.get("value") or ""
        ensure(group(r, 1) == value, f"{w}: shows {value!r}, not {group(r, 1)!r}")
        x, y = placed(e, b["base"], a["p"])
        ensure(close(float(group(r, 40)), a["height"] * float(e["scale"])), f"{w}: its height times the scale")
        turn = placed_turn(e, a["rotation"])
        got = float(group(r, 50) or 0.0)
        ensure(close(got, turn) or (turn == 0.0 and group(r, 50) is None), f"{w}: turned {turn}°, not {got}°")
        shown_at = {**a, "height": float(group(r, 40)), "rotation": got}
        if a.get("align") is None:
            ensure(close(float(group(r, 10)), x) and close(float(group(r, 20)), y), f"{w}: placed at {x}, {y}")
            ensure(group(r, 11) is None, f"{w}: not justified")
        else:
            check_place(r, (x, y), False, shown_at, "AcDbAttribute", 74, value, widths, w)


def check_insert(o: list[tuple[int, str]], e: dict, name: str, where: str) -> None:
    ensure(group(o, 2) == name, f"{where}: places {name!r}, not {group(o, 2)!r}")
    ensure(float(group(o, 10)) == e["p"]["x"] and float(group(o, 20)) == e["p"]["y"], f"{where}: its point")
    s = float(e["scale"])
    y = -s if e.get("mirror") else s
    ensure([float(group(o, c)) for c in (41, 42, 43)] == [s, y, s], f"{where}: scales {s}, {y}, {s}")
    degrees = float(group(o, 50))
    want = e["rotation"] * 180.0 / math.pi
    ensure(abs(degrees - want) <= 1e-12 * max(1.0, abs(want)), f"{where}: turned {want}°, not {degrees}°")
    turn = kentos(o).get("turn")
    if turn_of(degrees) == e["rotation"]:
        ensure(turn is None, f"{where}: the degrees give the turn back; no KentOS turn needed")
    else:
        ensure(turn is not None and float(turn[0]) == e["rotation"], f"{where}: KentOS's exact turn {e['rotation']}")


def dim_item(o: list[tuple[int, str]]) -> dict | None:
    """KentOS's "dimension" item: its style, offset and height, then its tagged lists (tag -> values)."""
    at = next((i for i, g in enumerate(o) if g == (1001, "KENTOS")), None)
    if at is None:
        return None
    k = next((i for i in range(at, len(o) - 1) if o[i] == (1002, "{") and o[i + 1] == (1000, "dimension")), None)
    if k is None:
        return None
    style, offset, height = o[k + 2][1], float(o[k + 3][1]), float(o[k + 4][1])
    lists: dict[str, list[str]] = {}
    i = k + 5
    while i < len(o) and o[i] != (1002, "}"):
        if o[i] == (1002, "{"):
            tag = o[i + 1][1]
            values = []
            i += 2
            while o[i] != (1002, "}"):
                values.append(o[i][1])
                i += 1
            lists[tag] = values
        i += 1
    return {"style": style, "offset": offset, "height": height, "lists": lists}


def dstyle(o: list[tuple[int, str]]) -> dict[int, str]:
    """ACAD's DSTYLE overrides of an entity: a variable's code -> its value."""
    at = next((i for i, g in enumerate(o) if g == (1000, "DSTYLE")), None)
    out: dict[int, str] = {}
    if at is None:
        return out
    i = at + 2
    while o[i] != (1002, "}"):
        out[int(o[i][1])] = o[i + 1][1]
        i += 2
    return out


def point(o: list[tuple[int, str]], code: int, marker: str | None = None) -> tuple[float, float]:
    """The point of group `code` (x) and `code + 10` (y), after the subclass `marker` when given."""
    k = after(o, marker) if marker else 0
    x = next(float(v) for c, v in o[k:] if c == code)
    y = next(float(v) for c, v in o[k:] if c == code + 10)
    return (x, y)


def xy(p: dict) -> tuple[float, float]:
    return (p["x"], p["y"])


def check_dimension(o: list[tuple[int, str]], e: dict, value: str | None, blocks: list, where: str) -> None:
    style = e.get("style")
    a, b = xy(e["a"]), xy(e["b"])
    entity = {"arcLength": "ARC_DIMENSION", "jogged": "LARGE_RADIAL_DIMENSION"}.get(style, "DIMENSION")
    ensure(o[0] == (0, entity), f"{where}: an {entity}")
    common = o[after(o, "AcDbDimension") :]
    flags = int(next(v for c, v in common if c == 70))
    over = dstyle(o)
    if style == "ordinate":
        east = (e.get("angle") or 0) == 0
        ensure(flags == 6 + 32 + (64 if east else 0), f"{where}: type 6, of the {'east' if east else 'north'} (70 {flags})")
        ensure(point(o, 10, "AcDbDimension") == (0.0, 0.0), f"{where}: measured from (0, 0)")
        ensure(point(o, 13, "AcDbOrdinateDimension") == a and point(o, 14, "AcDbOrdinateDimension") == b, f"{where}: its point and its line's end")
    elif style == "arcLength":
        c = xy(e["c"])
        ensure(flags == 5 + 32, f"{where}: type 5 (70 {flags})")
        m = "AcDbArcDimension"
        ensure(point(o, 13, m) == a and point(o, 14, m) == b and point(o, 15, m) == c, f"{where}: the arc's ends and centre")
        r = math.hypot(a[0] - c[0], a[1] - c[1])
        p10 = point(o, 10, "AcDbDimension")
        ensure(close(math.hypot(p10[0] - c[0], p10[1] - c[1]), r + e["offset"], 1e-9 * (r + 1)), f"{where}: 10 on the dimension arc")
        ta = math.atan2(a[1] - c[1], a[0] - c[0]) % math.tau
        tb = math.atan2(b[1] - c[1], b[0] - c[0]) % math.tau
        mid = ta + ((tb - ta) % math.tau) / 2
        t10 = math.atan2(p10[1] - c[1], p10[0] - c[0])
        ensure(close(math.cos(t10), math.cos(mid), 1e-9) and close(math.sin(t10), math.sin(mid), 1e-9), f"{where}: 10 halfway round")
        sub = o[after(o, m) :]
        ensure(close(float(group(sub, 40)), ta, 1e-12) and close(float(group(sub, 41)), tb, 1e-12), f"{where}: its ends' angles (40, 41)")
        ensure(group(sub, 70) == "0" and group(sub, 71) == "0", f"{where}: not partial, no leader")
    elif style == "jogged":
        c = xy(e["c"])
        ensure(flags == 9 + 32, f"{where}: type 9 (70 {flags})")
        m = "AcDbRadialDimensionLarge"
        ensure(point(o, 10, "AcDbDimension") == a, f"{where}: the true centre (10)")
        ensure(point(o, 13, m) == c and point(o, 15, m) == b, f"{where}: the centre shown (13), the point on the arc (15)")
        r = math.hypot(b[0] - a[0], b[1] - a[1])
        u = ((b[0] - a[0]) / r, (b[1] - a[1]) / r)
        nrm = (-u[1], u[0])
        side = (c[0] - b[0]) * nrm[0] + (c[1] - b[1]) * nrm[1]
        room = (b[0] - c[0]) * u[0] + (b[1] - c[1]) * u[1] - abs(side)
        along = min(max(e["offset"], 0.0), max(room, 0.0)) + abs(side) / 2
        jog = (c[0] + u[0] * along - nrm[0] * side / 2, c[1] + u[1] * along - nrm[1] * side / 2)
        got = point(o, 14, m)
        ensure(close(got[0], jog[0], 1e-8) and close(got[1], jog[1], 1e-8), f"{where}: the jog's middle (14)")
        ensure(float(group(o[after(o, m):], 40)) == 0.0, f"{where}: 40 0")
        ensure(close(float(over.get(50, "nan")), math.pi / 4, 1e-15), f"{where}: the jog's 45° (DSTYLE 50)")
    elif style == "radius":
        # A radius (type 4): its centre (10), the point on the arc (15), the leader past it (40).
        ensure(flags == 4 + 32, f"{where}: a radius (70 {flags})")
        ensure(point(o, 10, "AcDbDimension") == a and point(o, 15, "AcDbRadialDimension") == b, f"{where}: its centre and the point on the arc")
        ensure(float(group(o[after(o, "AcDbRadialDimension") :], 40)) == max(e["offset"], 0.0), f"{where}: its leader (40)")
    else:
        ensure(flags == 1 + 32, f"{where}: an aligned one (70 {flags})")
        ensure(point(o, 13, "AcDbAlignedDimension") == a and point(o, 14, "AcDbAlignedDimension") == b, f"{where}: its two points")
    if style != "jogged":
        ensure(50 not in over, f"{where}: no jog")
    item = dim_item(o)
    ensure(item is not None, f"{where}: KentOS's dimension item")
    ensure(item["style"] == (style or "") and item["offset"] == e["offset"] and item["height"] == e["height"], f"{where}: its style, offset and height")
    zs = {k: float(v[0]) for k, v in item["lists"].items() if k in ("za", "zb")}
    want = {k: e[k] for k in ("za", "zb") if style == "slope" and e.get(k) is not None}
    ensure(zs == want, f"{where}: its elevations {want}")
    masked = bool(e.get("mask"))
    ensure(("mask" in kentos(o)) == masked, f"{where}: KentOS's mask item exactly when masked")
    ensure((over.get(69) == "1") == masked, f"{where}: DIMTFILL 1 exactly when masked")
    name = group(o, 2)
    k = next(i for i, x in enumerate(blocks) if x[0] == (0, "BLOCK") and group(x, 2) == name)
    end = next(i for i in range(k, len(blocks)) if blocks[i][0] == (0, "ENDBLK"))
    mtexts = [x for x in blocks[k + 1 : end] if x[0] == (0, "MTEXT")]
    ensure(len(mtexts) == 1, f"{where}: its block's value")
    ensure((group(mtexts[0], 90) == "3") == masked, f"{where}: its value over the background exactly when masked")
    ensure(value is None or group(mtexts[0], 1) == value, f"{where}: its value {value!r}")


# Text and dimension styles (docs/adr/0183 §7).
FONTS = ["barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono"]
LABELS = {
    "barlow": "Barlow", "arimo": "Arimo", "overpass": "Overpass", "quicksand": "Quicksand",
    "architects-daughter": "Architects Daughter", "courier-prime": "Courier Prime", "plex-mono": "IBM Plex Mono",
}
STYLE_REFUSED = set('<>/\\":;?*|=,`')
FACE_KEYS = ("textStyle", "font", "bold", "italic", "oblique")
LOOK_KEYS = ("dimStyle", "arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit",
             "prefix", "suffix", "font")
STANDARD_STYLE = [(2, "Standard"), (70, "0"), (40, "0.0"), (41, "1.0"), (50, "0.0"), (71, "0"), (42, "2.5"),
                  (3, "arial.ttf"), (4, "")]


def font_file(font: str, bold: bool, italic: bool) -> str:
    style = {(False, False): "", (True, False): "bd", (False, True): "i", (True, True): "bi"}[(bold, italic)]
    if font == "arimo":
        return f"arial{style}.ttf"
    if font == "courier-prime":
        return f"cour{style}.ttf"
    return LABELS[font].replace(" ", "") + ".ttf"


def family(font: str) -> str:
    return {"arimo": "Arial", "courier-prime": "Courier New"}.get(font, LABELS[font])


def table_name(name: str) -> str:
    n = "".join("_" if (c in STYLE_REFUSED or unicodedata.category(c) == "Cc") else c for c in name.strip())
    return n or "Stil"


def style_names(spec: dict) -> tuple[dict, dict, dict]:
    """The records' names: the text styles', the dimension styles' (by id), the styleless typefaces' (by family, bold, italic)."""
    taken = ["STANDARD"]

    def unique(name: str) -> str:
        base = table_name(name)
        n, k = base, 2
        while n.upper() in taken:
            n = f"{base} {k}"
            k += 1
        taken.append(n.upper())
        return n

    text = {s["id"]: unique(s["name"]) for s in spec.get("textStyles", [])}
    dims = {s["id"]: unique(s["name"]) for s in spec.get("dimensionStyles", [])}
    synthetic: dict = {}
    for e in spec["entities"] + [x for b in spec["blocks"] for x in b["entities"]]:
        face = None
        # A table's cells are in its face, as a text's (docs/adr/0184 §7).
        if e["kind"] in ("text", "table") and e.get("textStyle") not in text and e.get("font"):
            face = (e["font"], bool(e.get("bold")), bool(e.get("italic")))
        elif e["kind"] == "dimension" and e.get("font"):
            face = (e["font"], False, False)
        if face and face not in synthetic:
            synthetic[face] = unique("KENTOS_" + face[0].replace("-", "_").upper() + ("_B" if face[1] else "") + ("_I" if face[2] else ""))
    return text, dims, synthetic


def item_text(values: list[str]) -> str:
    """One KentOS item's string: a single value, or its pieces in a nested list; carets decoded."""
    parts = values[1:] if values and values[0] == "{" else values[:1]
    return caret_decode("".join(parts))


def own(e: dict, keys: tuple) -> dict:
    return {k: e[k] for k in keys if e.get(k) is not None and e.get(k) is not False}


def check_style_tables(spec: dict, p: list[tuple[int, str]], names: tuple, paper: float, per_metre: float) -> list[str]:
    text, dims, synthetic = names
    length = lambda mm: mm / 1000.0 * paper
    records = split(section(p, "TABLES"))
    styles = [r for r in records if r[0] == (0, "STYLE")]
    given = spec.get("textStyles", [])
    faces = sorted(synthetic, key=lambda k: (FONTS.index(k[0]), k[1], k[2]))
    want = ["Standard"] + [text[s["id"]] for s in given] + [synthetic[k] for k in faces]
    ensure([group(r, 2) for r in styles] == want, f"STYLE records {want}")
    ensure(styles[0][styles[0].index((2, "Standard")) :] == STANDARD_STYLE, "Standard: as it always was, no extended data")

    def acad(r: list[tuple[int, str]]) -> list[tuple[int, str]]:
        at = r.index((1001, "ACAD"))
        return r[at + 1 : at + 3]

    def flags(bold: bool, italic: bool) -> str:
        return str(34 | (0x2000000 if bold else 0) | (0x1000000 if italic else 0))

    for s, r in zip(given, styles[1:]):
        w = f"yazı stili {s['name']}"
        height = length(s["height"]) if s.get("height") else 0.0
        ensure(close(float(group(r, 40)), height, 1e-12), f"{w}: its fixed height {height} in drawing units (40)")
        ensure(float(group(r, 41)) == s.get("widthFactor", 1.0) and float(group(r, 50)) == s.get("oblique", 0.0), f"{w}: its width and slant")
        ensure(group(r, 3) == s.get("fontFile", font_file(s["font"], bool(s.get("bold")), bool(s.get("italic")))), f"{w}: its typeface's file")
        ensure(acad(r) == [(1000, family(s["font"])), (1071, flags(bool(s.get("bold")), bool(s.get("italic"))))], f"{w}: ACAD's family and flags")
        ensure(json.loads(item_text(kentos(r)["face"])) == s, f"{w}: the style itself in KentOS's data")
    for (font, bold, italic), r in zip(faces, styles[1 + len(given) :]):
        w = f"yazı tipi {synthetic[(font, bold, italic)]}"
        ensure((float(group(r, 40)), float(group(r, 41)), float(group(r, 50))) == (0.0, 1.0, 0.0), f"{w}: no fixed height, width 1, no slant")
        ensure(group(r, 3) == font_file(font, bold, italic) and acad(r) == [(1000, family(font)), (1071, flags(bold, italic))], f"{w}: its file, family and flags")
        ensure(item_text(kentos(r)["face"]) == "styleless", f"{w}: KentOS's mark of no style")

    dimstyles = [r for r in records if r[0] == (0, "DIMSTYLE")]
    given = spec.get("dimensionStyles", [])
    want = ["Standard"] + [dims[s["id"]] for s in given]
    ensure([group(r, 2) for r in dimstyles] == want, f"DIMSTYLE records {want}")
    ensure((1001, "KENTOS") not in dimstyles[0], "Standard: no KentOS data")
    for s, r in zip(given, dimstyles[1:]):
        w = f"ölçü stili {s['name']}"
        h = length(s["height"])
        look = {k: (v / s["height"] if k in ("arrowSize", "extOffset", "extBeyond", "textGap") else v) for k, v in s.items() if k in LOOK_KEYS}
        check_vars({c: v for c, v in r if c not in (1001, 1000, 1002)}, look, h, s.get("decimals", 2), per_metre, w)
        flags_at = r.index((70, "0"))
        post = dimpost(look)
        ensure(post is None or r[flags_at + 1] == (3, post), f"{w}: DIMPOST {post!r} after the flags")
        ensure(json.loads(item_text(kentos(r)["look"])) == s, f"{w}: the style itself in KentOS's data")
    return [f"{len(spec.get('textStyles', []))} yazı stili, {len(synthetic)} yazı tipi kaydı, {len(given)} ölçü stili"] if (given or spec.get("textStyles") or synthetic) else []


def dimpost(look: dict) -> str | None:
    if look.get("prefix") is None and look.get("suffix") is None:
        return None
    return f"{look.get('prefix', '')}<>{look.get('suffix', '')}"


def check_vars(v: dict, look: dict, h: float, decimals: int, per_metre: float, where: str) -> None:
    """A look's variables (a DIMSTYLE record's groups or a DIMENSION's DSTYLE overrides, code -> value) at height h."""
    tick = "arrow" not in look
    size = look.get("arrowSize", 0.6 if tick else 1.0) * h
    real = lambda code: float(v[code]) if code in v else None
    ensure(close(real(140), h, 1e-12), f"{where}: its value's height (140)")
    if tick:
        ensure(close(real(142), size * math.sqrt(0.5), 1e-12), f"{where}: ticks, half a 45° tick of {size} (DIMTSZ)")
    else:
        ensure(real(142) == 0.0, f"{where}: no ticks (DIMTSZ 0)")
        ensure(close(real(41), 0.0 if look["arrow"] == "none" else size, 1e-12), f"{where}: its arrowheads' size (DIMASZ; 0 for none)")
    ensure(close(real(42), look.get("extOffset", 0.5) * h, 1e-12), f"{where}: the extension lines' offset (42)")
    ensure(close(real(44), look.get("extBeyond", 0.5) * h, 1e-12), f"{where}: the extension lines' reach (44)")
    ensure(close(real(147), look.get("textGap", 0.35) * h, 1e-12), f"{where}: the value's gap (147)")
    ensure(v.get(77) == ("0" if look.get("textPlace") == "centre" else "1"), f"{where}: the value's place (77)")
    ensure(v.get(271) == str(min(look.get("decimals", decimals), 8)), f"{where}: its decimals (271)")
    post = dimpost(look)
    ensure(v.get(3) == post if post is not None else (v.get(3) is None), f"{where}: DIMPOST {post!r}")
    factor = {"m": 1.0, "cm": 100.0, "mm": 1000.0}[look["unit"]] / per_metre if look.get("unit") else 1.0
    if factor != 1.0:
        ensure(close(real(144), factor, 1e-12), f"{where}: DIMLFAC {factor}")
    else:
        ensure(real(144) in (None, 1.0), f"{where}: no DIMLFAC of its own")


def check_face(o: list[tuple[int, str]], e: dict, names: tuple, mtext: bool, where: str) -> None:
    """A TEXT's or an MTEXT's style record (7), own slant (51) and face in KentOS's data."""
    text, _, synthetic = names
    if e.get("textStyle") in text:
        record = text[e["textStyle"]]
    elif e.get("font"):
        record = synthetic[(e["font"], bool(e.get("bold")), bool(e.get("italic")))]
    else:
        record = "Standard"
    if mtext or record != "Standard":
        ensure(group(o, 7) == record, f"{where}: names its style's record {record!r} (7)")
    else:
        ensure(group(o, 7) is None, f"{where}: Standard left unsaid")
    if not mtext:
        slant = e.get("oblique") if e.get("font") else None
        ensure((group(o, 51) is None) if slant is None else float(group(o, 51)) == slant, f"{where}: its own slant {slant} (51)")
    face = own(e, FACE_KEYS)
    got = kentos(o).get("face")
    ensure((got is None) if not face else (got is not None and json.loads(item_text(got)) == face), f"{where}: its face {face} in KentOS's data")


def arrow_ends(e: dict) -> list[tuple[tuple[float, float], tuple[float, float]]]:
    """A dimension's arrowheads: each tip and the way into the dimension line (aligned and radius dimensions)."""
    a, b = xy(e["a"]), xy(e["b"])
    style = e.get("style")
    if style in (None, "aligned"):
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        u = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
        n = (-u[1], u[0])
        off = e["offset"]
        d1 = (a[0] + n[0] * off, a[1] + n[1] * off)
        k = off + (n[0] * (a[0] - b[0]) + n[1] * (a[1] - b[1]))
        d2 = (b[0] + n[0] * k, b[1] + n[1] * k)
        l = math.hypot(d2[0] - d1[0], d2[1] - d1[1])
        along = ((d2[0] - d1[0]) / l, (d2[1] - d1[1]) / l)
        return [(d1, along), (d2, (-along[0], -along[1]))]
    if style == "radius":
        r = math.hypot(b[0] - a[0], b[1] - a[1])
        return [(b, (-(b[0] - a[0]) / r, -(b[1] - a[1]) / r))]
    raise Bad(f"no arrowhead rule here for a {style} dimension")


def check_dim_look(o: list[tuple[int, str]], e: dict, names: tuple, spec: dict, blocks: list, per_metre: float, where: str) -> None:
    """A DIMENSION's style record (3), its look in DSTYLE and KentOS's data, its block's arrowheads and value's record."""
    _, dims, synthetic = names
    look = own(e, LOOK_KEYS)
    ensure(group(o[after(o, "AcDbDimension") :], 3) == dims.get(e.get("dimStyle"), "Standard"), f"{where}: names its style's record (3)")
    check_vars(dstyle(o), look, e["height"], spec["lengthDecimals"], per_metre, f"{where} (DSTYLE)")
    got = kentos(o).get("look")
    ensure((got is None) if not look else (got is not None and json.loads(item_text(got)) == look), f"{where}: its look {look} in KentOS's data")
    name = group(o, 2)
    k = next(i for i, x in enumerate(blocks) if x[0] == (0, "BLOCK") and group(x, 2) == name)
    end = next(i for i in range(k, len(blocks)) if blocks[i][0] == (0, "ENDBLK"))
    inside = blocks[k + 1 : end]
    mtext = next(x for x in inside if x[0] == (0, "MTEXT"))
    value = synthetic[(e["font"], False, False)] if e.get("font") else "Standard"
    ensure(group(mtext, 7) == value, f"{where}: its value in its typeface's record {value!r}")
    solids = [x for x in inside if x[0] == (0, "SOLID")]
    dots = [x for x in inside if x[0] == (0, "LWPOLYLINE")]
    arrow = look.get("arrow")
    size = look.get("arrowSize", 0.6 if arrow is None else 1.0) * e["height"]
    near = lambda q, r: abs(q[0] - r[0]) <= 1e-9 * max(1.0, abs(r[0])) and abs(q[1] - r[1]) <= 1e-9 * max(1.0, abs(r[1]))
    ends = arrow_ends(e) if arrow in ("closed", "dot", "open") else []
    ensure(len(solids) == (len(ends) if arrow == "closed" else 0), f"{where}: a SOLID for each filled arrowhead")
    ensure(len(dots) == (len(ends) if arrow == "dot" else 0), f"{where}: a donut for each dot")
    lines = [(point(x, 10), point(x, 11)) for x in inside if x[0] == (0, "LINE")]
    for tip, into in ends:
        nrm = (-into[1], into[0])
        base = (tip[0] + into[0] * size, tip[1] + into[1] * size)
        left = (base[0] + nrm[0] * size / 6, base[1] + nrm[1] * size / 6)
        right = (base[0] - nrm[0] * size / 6, base[1] - nrm[1] * size / 6)
        if arrow == "closed":
            ok = any(near(point(s, 10), tip) and near(point(s, 11), left) and near(point(s, 12), right) and point(s, 13) == point(s, 12) for s in solids)
            ensure(ok, f"{where}: a filled arrowhead at {tip}")
        elif arrow == "dot":
            r = size / 4
            def donut(d: list[tuple[int, str]]) -> bool:
                xs = [float(v) for c, v in d if c == 10]
                ys = [float(v) for c, v in d if c == 20]
                return (group(d, 90) == "2" and group(d, 70) == "1" and abs(float(group(d, 43)) - r) <= 1e-9
                        and [float(v) for c, v in d if c == 42] == [1.0, 1.0]
                        and near((xs[0] + r / 2, ys[0]), tip) and near((xs[1] - r / 2, ys[1]), tip))
            ensure(any(donut(d) for d in dots), f"{where}: a dot of radius {r} at {tip}")
        else:
            ensure(any(near(p, left) and near(q, tip) for p, q in lines) and any(near(p, tip) and near(q, right) for p, q in lines), f"{where}: an open arrowhead at {tip}")


def check(name: str) -> list[str]:
    spec = json.loads((DIR / f"{name}.input.json").read_text(encoding="utf-8"))
    p = pairs((DIR / f"{name}.dxf").read_bytes())
    # The drawing in the file's unit: a local project's (docs/adr/0165 §2), else metres.
    unit = spec.get("unit") or "m"
    per_metre, code = UNITS[unit]
    head = section(p, "HEADER")
    at = head.index((9, "$INSUNITS"))
    ensure(head[at + 1] == (70, code), f"$INSUNITS {code} ({unit})")
    if per_metre != 1.0:
        spec = scaled(spec, per_metre)
    layers = {l["id"]: l["name"] for l in spec["layers"]}
    order = walk(spec["blocks"], spec["entities"])
    names = dxf_names(order)
    said = [f"birim {unit} ($INSUNITS {code})"] if unit != "m" else []
    widths: dict = {}
    # The style records' names (docs/adr/0183 §7); a style's paper mm in the file's unit.
    styled = style_names(spec)
    said.extend(check_style_tables(spec, p, styled, spec["scale"] * per_metre, per_metre))

    # Records and BLOCKs, in the walk's order after model and paper space.
    tables = section(p, "TABLES")
    records = [r for r in split(tables) if r[0] == (0, "BLOCK_RECORD")]
    record_of = {group(r, 2): group(r, 5) for r in records}
    # Each dimension's own anonymous block (*D1, *D2 …): a block's dimensions' before it (blocks it holds), the
    # drawing's after the drawing's blocks (docs/adr/0147 §8).
    want, k = ["*Model_Space", "*Paper_Space"], 0
    for b in order:
        for e in b["entities"]:
            if e["kind"] == "dimension":
                k += 1
                want.append(f"*D{k}")
        want.append(names[b["id"]])
    # The drawing's dimensions' blocks and its tables' (*U1, *U2 …, docs/adr/0184 §7), in the drawing's order.
    u = 0
    for e in spec["entities"]:
        if e["kind"] == "dimension":
            k += 1
            want.append(f"*D{k}")
        elif e["kind"] == "table":
            u += 1
            want.append(f"*U{u}")
    ensure([group(r, 2) for r in records] == want, f"block records {want}")
    blocks = split(section(p, "BLOCKS"))
    heads = [e for e in blocks if e[0] == (0, "BLOCK")]
    ensure([group(h, 2) for h in heads] == want, f"BLOCKs {want}")
    for b in spec["blocks"]:
        if b["id"] not in names:
            ensure(all(group(h, 2) != b["name"] for h in heads), f"the unused {b['name']!r} is not written")

    # Each block's head, objects and end.
    i = 0
    for b in order:
        while blocks[i][0] != (0, "BLOCK") or group(blocks[i], 2) != names[b["id"]]:
            i += 1
        head = blocks[i]
        rec = record_of[names[b["id"]]]
        ensure(group(head, 330) == rec, f"{b['name']}: the BLOCK is its record's")
        ensure(float(group(head, 10)) == b["base"]["x"] and float(group(head, 20)) == b["base"]["y"], f"{b['name']}: base point")
        flags = "2" if b.get("attributes") else "0"
        ensure(group(head, 70) == flags, f"{b['name']}: not anonymous, flagged {flags} (attribute definitions)")
        if b.get("description"):
            one = "".join(" " if ord(c) < 32 else c for c in b["description"])
            ensure(group(head, 4) == one, f"{b['name']}: its description on one line")
        end = next(k for k in range(i, len(blocks)) if blocks[k][0] == (0, "ENDBLK"))
        # Its attribute definitions follow its objects.
        attributes = b.get("attributes") or []
        defs = blocks[end - len(attributes) : end]
        objects = blocks[i + 1 : end - len(attributes)]
        ensure(group(blocks[end], 330) == rec, f"{b['name']}: ENDBLK is its record's")
        for a, tag, d in zip(attributes, dxf_tags(b), defs):
            w = f"{b['name']} › {a['tag']}"
            ensure(d[0] == (0, "ATTDEF") and group(d, 330) == rec and group(d, 8) == "0", f"{w}: an ATTDEF of its record on 0")
            ensure(float(group(d, 40)) == a["height"], f"{w}: its height")
            check_place(d, (a["p"]["x"], a["p"]["y"]), True, a, "AcDbAttributeDefinition", 74, tag, widths, w)
            ensure(group(d, 1) == (a.get("value") or ""), f"{w}: its default")
            ensure(group(d, 3) == (a.get("prompt") or ""), f"{w}: its prompt")
            ensure(group(d, 2) == tag and group(d, 70) == "0", f"{w}: tag {tag!r}, visible")
            ensure((group(d, 50) is None) if a["rotation"] == 0 else float(group(d, 50)) == a["rotation"], f"{w}: its turn")
        kinds = [k for e in b["entities"] for k in written_kinds(e)]
        ensure([o[0][1] for o in objects] == kinds, f"{b['name']}: objects {kinds}")
        k = 0
        for n, e in enumerate(b["entities"]):
            where = f"{b['name']} › {n + 1} ({e['kind']})"
            o = objects[k]
            ensure(group(o, 330) == rec, f"{where}: belongs to its block")
            check_look(o, e, layers, where)
            if e["kind"] == "insert":
                check_insert(o, e, names[e["block"]], where)
            if e["kind"] == "leader":
                check_leader(o, objects[k + 1] if note_of(e) else None, e, where)
                if note_of(e):
                    check_look(objects[k + 1], e, layers, f"{where} › MTEXT")
            if is_paragraph(e):
                check_paragraph(o, e, where)
            if e["kind"] == "text":
                check_face(o, e, styled, is_paragraph(e), where)
            if e["kind"] == "dimension":
                check_dimension(o, e, e.get("text"), blocks, where)
                check_dim_look(o, e, styled, spec, blocks, per_metre, where)
            if e["kind"] == "polygon":
                own = group(o, 5)
                for h in objects[k + 1 : k + len(written_kinds(e))]:
                    ensure(kentos(h).get("hole") == [own], f"{where}: its hole names it")
            k += len(written_kinds(e))
        said.append(f"{names[b['id']]}: {len(objects)} nesne" + (f" ve {len(attributes)} ATTDEF" if attributes else ""))
        i = end

    # The drawing's inserts, each with its ATTRIBs after it; a table's INSERT names an anonymous block (*U).
    drawn = split(section(p, "ENTITIES"))
    at = [k for k, e in enumerate(drawn) if e[0] == (0, "INSERT") and not (group(e, 2) or "").startswith("*U")]
    given = [e for e in spec["entities"] if e["kind"] == "insert" and e["block"] in names]
    ensure(len(at) == len(given), f"{len(given)} inserts written, the unknown block's left out")
    by_id = {b["id"]: b for b in spec["blocks"]}
    attributed = 0
    for n, (k, e) in enumerate(zip(at, given)):
        o = drawn[k]
        where = f"yerleştirme {n + 1}"
        ensure(group(o, 330) == "17", f"{where}: in model space")
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its layer")
        check_insert(o, e, names[e["block"]], where)
        b = by_id[e["block"]]
        if b.get("attributes"):
            ensure(group(o, 66) == "1", f"{where}: attributes follow (66 1)")
            check_attributes(drawn[k + 1 :], e, b, layers[e["layerId"]], group(o, 5), where, widths)
            attributed += 1
        else:
            ensure(group(o, 66) is None, f"{where}: no attributes follow")
        # The values of its block's attributes go out once, as its ATTRIBs; KentOS's data holds the others.
        defined = {a["tag"] for a in b.get("attributes") or []}
        others = {k: v for k, v in e["attrs"].items() if k not in defined}
        ensure(kentos_attrs(o) == others, f"{where}: KentOS's data holds its other attributes {others}")
    if attributed:
        said.append(f"{attributed} yerleştirmenin ATTRIB'leri")

    # The drawing's tables (docs/adr/0184 §7), in the input's order.
    tables = [e for e in spec["entities"] if e["kind"] == "table"]
    written = [e for e in drawn if e[0] == (0, "INSERT") and (group(e, 2) or "").startswith("*U")]
    ensure(len(written) == len(tables), f"{len(tables)} tables")
    by_name = {group(b, 2): k for k, b in enumerate(blocks) if b[0] == (0, "BLOCK")}
    for n, (e, o) in enumerate(zip(tables, written)):
        check_table(o, e, blocks, by_name, layers, f"tablo {n + 1}", n + 1)
    if tables:
        said.append(f"{len(tables)} tablo ({sum(1 for e in tables if e.get('frame') and e.get('grid') != 'none')} kalın çerçeveli)")

    # The drawing's texts (docs/adr/0145 §7), in the input's order.
    texts = [e for e in spec["entities"] if e["kind"] == "text" and not is_paragraph(e)]
    written = [e for e in drawn if e[0] == (0, "TEXT")]
    ensure(len(written) == len(texts), f"{len(texts)} TEXTs")
    for n, (e, o) in enumerate(zip(texts, written)):
        where = f"yazı {n + 1} ({e['text']})"
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its layer")
        ensure(group(o, 1) == e["text"] and float(group(o, 40)) == e["height"], f"{where}: its words and height")
        ensure((group(o, 50) is None) if e["rotation"] == 0 else float(group(o, 50)) == e["rotation"], f"{where}: its turn")
        check_place(o, (e["p"]["x"], e["p"]["y"]), True, e, "AcDbText", 73, e["text"], widths, where)
        ensure(("mask" in kentos(o)) == bool(e.get("mask")), f"{where}: KentOS's mask item exactly when masked")
        check_face(o, e, styled, False, where)
    check_widths(widths)
    if texts:
        aligned = sum(1 for e in texts if e.get("align"))
        said.append(f"{len(texts)} yazı ({aligned} hizalı, {sum(1 for e in texts if e.get('mask'))} zeminli)")

    # The drawing's multi-line texts (docs/adr/0182 §5), in the input's order: MTEXTs that react to nothing.
    paragraphs = [e for e in spec["entities"] if is_paragraph(e)]
    written = [e for e in drawn if e[0] == (0, "MTEXT") and (102, "{ACAD_REACTORS") not in e]
    ensure(len(written) == len(paragraphs), f"{len(paragraphs)} MTEXTs")
    for n, (e, o) in enumerate(zip(paragraphs, written)):
        where = f"çok satırlı yazı {n + 1}"
        ensure(group(o, 330) == "17" and group(o, 8) == layers[e["layerId"]], f"{where}: in model space, on its layer")
        check_paragraph(o, e, where)
        check_face(o, e, styled, True, where)
    if paragraphs:
        said.append(f"{len(paragraphs)} çok satırlı yazı ({sum(len(e.get('runs') or []) for e in paragraphs)} biçim dilimi)")

    # The drawing's leaders (docs/adr/0146 §8), in the input's order, each with its MTEXT after it.
    leaders = [e for e in spec["entities"] if e["kind"] == "leader"]
    at = [k for k, e in enumerate(drawn) if e[0] == (0, "LEADER")]
    ensure(len(at) == len(leaders), f"{len(leaders)} LEADERs")
    for n, (k, e) in enumerate(zip(at, leaders)):
        where = f"kılavuz {n + 1}"
        o = drawn[k]
        ensure(group(o, 330) == "17" and group(o, 8) == layers[e["layerId"]], f"{where}: in model space, on its layer")
        if e.get("color"):
            ensure(group(o, 420) == str(int(e["color"][1:], 16)), f"{where}: its true colour")
        check_leader(o, drawn[k + 1] if note_of(e) else None, e, where)
    if leaders:
        noted = sum(1 for e in leaders if note_of(e))
        said.append(f"{len(leaders)} kılavuz ({noted} notlu MTEXT'iyle)")

    # The drawing's dimensions (docs/adr/0147 §8), in the input's order.
    dims = [e for e in spec["entities"] if e["kind"] == "dimension"]
    kinds = ("DIMENSION", "ARC_DIMENSION", "LARGE_RADIAL_DIMENSION")
    written = [e for e in drawn if e[0][0] == 0 and e[0][1] in kinds]
    ensure(len(written) == len(dims), f"{len(dims)} dimensions")
    by_name = {group(b, 2): b for b in blocks if b[0] == (0, "BLOCK")}
    for n, (e, o) in enumerate(zip(dims, written)):
        where = f"ölçü {n + 1} ({e.get('style', 'aligned')})"
        check_dimension(o, e, spec["dimensionValues"].get(str(e["id"])), blocks, where)
        check_dim_look(o, e, styled, spec, blocks, per_metre, where)
    if dims:
        said.append(f"{len(dims)} ölçü ({sum(1 for e in dims if e.get('mask'))} zeminli)")

    # The drawing's points and polylines (docs/adr/0174 §5), in the input's order: DXF has no multi-part line or
    # multi-point object, so each point of a multi-point object is a POINT and each part of a multi-part polyline a
    # polyline (a 3D POLYLINE of VERTEXes when the part has elevations and no arc, else an open LWPOLYLINE with its
    # arcs' bulges), every one on the object's layer with its attributes in KentOS's data.
    points = [(q, e) for e in spec["entities"] if e["kind"] == "point" for q in [e, *(e.get("parts") or [])]]
    written = [o for o in drawn if o[0] == (0, "POINT")]
    ensure(len(written) == len(points), f"{len(points)} POINTs")
    for n, ((q, e), o) in enumerate(zip(points, written)):
        where = f"nokta {n + 1}"
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its object's layer")
        ensure((float(group(o, 10)), float(group(o, 20))) == xy(q["p"]), f"{where}: its place")
        ensure(float(group(o, 30)) == float(q.get("z") or 0.0), f"{where}: its height (0 without one)")
        ensure(kentos_attrs(o) == e["attrs"], f"{where}: its object's attributes")
    paths = [(q, e) for e in spec["entities"] if e["kind"] == "polyline" for q in [e, *(e.get("parts") or [])]]
    # The open ones: an area's ring and its holes are closed (70 bit 1).
    heads = [k for k, o in enumerate(drawn) if o[0] in ((0, "LWPOLYLINE"), (0, "POLYLINE")) and int(group(o, 70) or "0") & 1 == 0]
    ensure(len(heads) == len(paths), f"{len(paths)} polylines")
    for n, ((q, e), k) in enumerate(zip(paths, heads)):
        o, where = drawn[k], f"çoklu çizgi parçası {n + 1}"
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its object's layer")
        want = [xy(p) for p in q["pts"]]
        bulges = q.get("bulges") or []
        if o[0] == (0, "POLYLINE"):
            ensure(not any(bulges) and q.get("zs") is not None, f"{where}: 3D only with elevations and no arc")
            vertices = []
            for v in drawn[k + 1 :]:
                if v[0] != (0, "VERTEX"):
                    break
                vertices.append(((float(group(v, 10)), float(group(v, 20))), float(group(v, 30))))
            ensure([p for p, _ in vertices] == want, f"{where}: its vertices")
            ensure([z for _, z in vertices] == [float(z) for z in q["zs"]], f"{where}: each vertex's height")
        else:
            got = [(float(a[1]), float(b[1])) for a, b in zip(o, o[1:]) if a[0] == 10 and b[0] == 20]
            ensure(got == want, f"{where}: its vertices")
            ensure([float(v) for c, v in o if c == 42] == [float(b) for b in bulges if b != 0], f"{where}: its arcs' bulges")
        ensure(kentos_attrs(o) == e["attrs"], f"{where}: its object's attributes")
    several = sum(1 for e in spec["entities"] if e["kind"] in ("point", "polyline") and e.get("parts"))
    if several:
        said.append(f"{several} çok parçalı nesne: {len(points)} POINT, {len(paths)} çoklu çizgi")

    # Handles unique; owners name handles of the file.
    body = p[next(k for k, g in enumerate(p) if g == (2, "CLASSES")) :]
    handles = [int(v, 16) for c, v in body if c in (5, 105)]
    ensure(len(handles) == len(set(handles)), "handles are unique")
    known = set(handles)
    for c, v in body:
        if c == 330 and v != "0":
            ensure(int(v, 16) in known, f"owner {v} is a handle of the file")
    return said


def main() -> int:
    if "--check" not in sys.argv[1:]:
        print(__doc__)
        return 2
    names = sorted(p.name[: -len(".input.json")] for p in DIR.glob("*.input.json"))
    failed = False
    for name in names:
        try:
            said = check(name)
            print(f"✓ {name}.dxf: {', '.join(said)}")
        except (Bad, StopIteration, KeyError, ValueError, TypeError) as e:
            failed = True
            print(f"✗ {name}.dxf: {e}")
    return 1 if failed or not names else 0


if __name__ == "__main__":
    sys.exit(main())
