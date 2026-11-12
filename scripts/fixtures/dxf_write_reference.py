#!/usr/bin/env python3
"""Independent check of the DXF writer's blocks (docs/adr/0144 §5), texts (docs/adr/0145 §7), leaders (docs/adr/0146 §8),
multi-part lines and points (docs/adr/0174 §5), multi-line texts (docs/adr/0182 §5), text and dimension styles
(docs/adr/0183 §7) and hatch patterns and gradients (docs/adr/0186 §9).

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
  of its own; BYBLOCK (62 0, 370 -2) where it has no colour or weight, and
  (docs/adr/0312 §4) where it has no line type (6 ByBlock) or transparency
  (440 16777216) and does not follow its layer for it;
- an INSERT names its block, sits at the insert's point, scales X and Z by
  the scale and Y by it or its negative (mirrored), turns by the rotation in
  degrees; KentOS's data carries the exact radians exactly when the degrees
  do not give them back; an insert of a block the input does not have is
  not written;
- a block with attribute definitions (docs/adr/0144 §7) is flagged so (70 2)
  and has an ATTDEF for each after its objects, on 0: its place, height,
  default, turn (none when 0), prompt and tag (white space and control
  characters "_", a tag taken before "_2"; such a tag's ATTDEF, and only it,
  holds KentOS's "tagname" item: the tag and the name, docs/adr/4800); an
  insert of it says attributes
  follow (66 1) and is followed by an ATTRIB for each, owned by it, on its
  layer: the text it shows (its value, else the default, else nothing),
  placed as the insert places the definition (height times the scale; the
  turn and alignment as a text's under the insert's similarity, a mirrored
  one readable by docs/adr/0292's rule, `affine_reference.text_frame`:
  its up or its baseline turned back, its alignment so, within 1e-9; its
  41 fitted to Arial, below), then its other attributes with a value of
  one line (docs/adr/3980), by name, as invisible ATTRIBs (70 1) at its
  point, 2.5 mm high, tagged as a definition is (a tag taken before, but
  for case, "_2" …; a tag that is not its name named in KentOS's data,
  docs/adr/4380), then a SEQEND of its own; an insert with such attributes
  alone says they follow too; KentOS's data holds the rest;
- a text (a TEXT, an ATTRIB, an ATTDEF; docs/adr/0145 §7) without an
  alignment starts at its point (10) and has no 11, 72 or 73; an aligned one
  stands on its alignment point (11) exactly (an ATTRIB's where its insert
  places it, within 1e-9), its 72 and 73 (an attribute's 74) name its
  alignment (none written for 0), 72 and 11 before the second subclass
  marker, the vertical after it; its start (10) is its alignment's share of
  its height under 11 across its baseline (within 1e-8 m) and its share of
  its width back along it, the width every text of the same words, height
  and width factor implies being one (within 1e-8 m) and an average letter
  of 0.3 to 0.9 of its height; that width is its line's in Arial (Arimo's
  advances) at its height times its 41 (within 1e-8 m): a program that does
  not justify draws it from 10 and ends at 11; its width factor is 41 (none for 1); a
  masked TEXT has KentOS's "mask" item, no other has;
- a TEXT, an ATTRIB, a table's cell and a curved text's letter in KentOS's
  own face (Standard, which other programs draw as Arial) have their 41
  fitted (docs/adr/8046 §2): the width factor times Barlow's width as
  KentOS lays the line out (bold as it draws it) over Arial's regular
  (Arimo's), to three places, the product to six; a TEXT's KentOS data says
  the fit ("fit"); an invisible ATTRIB is not fitted;
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
  (90 3). Its own changes to Standard (ACAD's DSTYLE data, docs/adr/0205
  §7) give its arrowhead's length (41: its size, else 1, times its height,
  exactly) and, for an arrowhead but the filled one and none, its DXF
  arrow block (341: the handle of the block record named _ClosedBlank,
  _Open, _Open30, _Open90, _Dot, _DotSmall, _DotBlank, _Oblique,
  _ArchTick, _BoxFilled, _BoxBlank or _DatumFilled), written once, its
  record and BLOCK where the first leader with it puts them (a block's
  before the block, the drawing's in its order), not anonymous (70 0), its
  objects on 0 in BYBLOCK's colour and weight: the arrowhead one long, its
  tip at the origin and its line along −x, as DXF's named ones are (a triangle
  of base a third of its length, the open arrow's sides 15° and 45° off
  the line, the dots of radius 1/4 and 1/8, the box 1/2 a side, the tick at
  45° 1/√2 long each way, the architectural tick a band 1/8 wide, the
  datum triangle's base 1 across the tip): a filled area a SOLID (a dot a
  donut: a closed LWPOLYLINE of two half circles, as wide as its radius),
  a line an LWPOLYLINE, a blank dot a CIRCLE. KentOS's data has no
  arrowhead ("arrow") and its size ("arrowsize") exactly when DIMASZ over
  the height does not give it back; it holds the note exactly when MTEXT
  cannot (a control character in it), and the turn, when it does,
  exactly;
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
- a hatch (docs/adr/0186 §9) is never associative (71 0), its tie left
  out; user lines and crossed lines are DXF's user-defined pattern "_USER"
  (76 0): its angle (52) and spacing (41), doubled when crossed (77 1), one
  family at the angle, the second a quarter turn on, each the spacing apart
  across itself, whole; a pattern is its name (2), predefined (76 1) when
  KentOS's library has the name, custom (76 2) else, its angle (52) and
  scale (41) as they are, not doubled, and each family turned and scaled as
  DXF holds it: its angle the pattern's plus its own (53), its base point
  its origin turned by the pattern and scaled (43, 44), from one line to the
  next its offset turned by its angle and scaled (45, 46), its dashes scaled
  (79, 49), within 1e-9; a gradient is a solid fill (2 SOLID, 70 1) of a
  gradient (450 1) by its DXF name (470; an inverted linear one LINEAR
  turned a half turn), its angle in radians (460), centred (461 0), two
  colours (453 2), the first the hatch's own (its layer's without one), the
  second the gradient's, each an ACI index (63) and its true colour (421:
  the colour itself, the ink ACI 7's white); a pattern's and a gradient's
  KentOS data hold it exactly ("hatch", the contract's JSON), a user
  pattern's never;
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
  its own, else Arial's and Courier's as DXF files name them, KentOS's
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
import struct
import sys
import unicodedata
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from affine_reference import text_frame  # noqa: E402  (docs/adr/0292's rule, written without KentOS code)

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
LENGTH_KEYS = {"r", "height", "offset", "z", "za", "zb", "boxWidth", "frame"}


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
            # Only lines are apart by a length; a pattern's metres per unit of its definition scale too.
            out[key] = dict(v)
            if v["type"] in ("lines", "cross"):
                out[key]["spacing"] = v["spacing"] * k
            if "scale" in v:
                out[key]["scale"] = v["scale"] * k
        elif key == "assoc":
            out[key] = {**v, "seed": xy(v["seed"])}
        elif key in ("rows", "columns") and isinstance(v, list) and all(isinstance(x, (int, float)) for x in v):
            # A table's rows' heights and columns' widths (docs/adr/0184 §1).
            out[key] = [x * k for x in v]
        elif key in ("entities", "blocks", "attributes", "parts", "path"):
            # A curved text's curve is in the text's own frame, metres (docs/adr/0196 §1): its points scale.
            out[key] = scaled(v, k)
        else:
            out[key] = v
    return out


# ── Arial's fit (docs/adr/8046 §2) ────────────────────────────────────
# A text in KentOS's own face (Barlow; no style, no typeface: Standard) is drawn as Arial by other programs: its 41 is
# its width factor times Barlow's width as KentOS lays the line out (the measured advances, text_along_cases.py; bold
# when KentOS draws it bold) over Arial's own (its metric twin Arimo's regular, read from the face's file here: Standard
# is arial.ttf, a bold line goes out regular), to three places, the product to six; a TEXT's KENTOS data says the
# factor. A letter Arimo has no glyph of leaves it as it was.
FACES = Path(__file__).resolve().parents[2] / "apps/desktop/assets/fonts/drawing"


def ttf_advances(path: Path) -> tuple[dict[int, int], int]:
    """A TrueType face's advances by code point (its Unicode cmap's format 4 subtable) in its units, and its units per
    em, read with the standard library."""
    data = path.read_bytes()
    tables = {}
    for i in range(struct.unpack(">H", data[4:6])[0]):
        tag, _, off, _ = struct.unpack(">4sIII", data[12 + 16 * i : 28 + 16 * i])
        tables[tag.decode("latin-1")] = off
    upm = struct.unpack(">H", data[tables["head"] + 18 : tables["head"] + 20])[0]
    metrics = struct.unpack(">H", data[tables["hhea"] + 34 : tables["hhea"] + 36])[0]
    hmtx = tables["hmtx"]
    advances = [struct.unpack(">H", data[hmtx + 4 * g : hmtx + 4 * g + 2])[0] for g in range(metrics)]
    cmap = tables["cmap"]
    sub = None
    for i in range(struct.unpack(">H", data[cmap + 2 : cmap + 4])[0]):
        platform, encoding, off = struct.unpack(">HHI", data[cmap + 4 + 8 * i : cmap + 12 + 8 * i])
        if (platform, encoding) in ((3, 1), (0, 3)) and struct.unpack(">H", data[cmap + off : cmap + off + 2])[0] == 4:
            sub = cmap + off
            break
    twice = struct.unpack(">H", data[sub + 6 : sub + 8])[0]
    n = twice // 2
    ends = struct.unpack(f">{n}H", data[sub + 14 : sub + 14 + twice])
    starts = struct.unpack(f">{n}H", data[sub + 16 + twice : sub + 16 + 2 * twice])
    deltas = struct.unpack(f">{n}h", data[sub + 16 + 2 * twice : sub + 16 + 3 * twice])
    ranges = sub + 16 + 3 * twice
    offsets = struct.unpack(f">{n}H", data[ranges : ranges + twice])
    out: dict[int, int] = {}
    for k in range(n):
        for cp in range(starts[k], ends[k] + 1):
            if cp == 0xFFFF:
                continue
            if offsets[k] == 0:
                g = (cp + deltas[k]) & 0xFFFF
            else:
                at = ranges + 2 * k + offsets[k] + 2 * (cp - starts[k])
                g = struct.unpack(">H", data[at : at + 2])[0]
                g = (g + deltas[k]) & 0xFFFF if g else 0
            if g:
                out[cp] = advances[min(g, metrics - 1)]
    return out, upm


ARIAL = ttf_advances(FACES / "Arimo-400.ttf")


def arial_fit(line: str, bold: bool) -> float | None:
    """Barlow's width (bold or not) over Arial's regular for `line`, to three places; None when Arimo has no glyph of
    a letter."""
    import text_along_cases as along

    glyphs, upm = ARIAL
    if any(ord(c) not in glyphs for c in line):
        return None
    ours = sum(along.advance("barlow", c, bold) for c in line) / 1000.0
    arial = sum(glyphs[ord(c)] for c in line) / upm
    if not (ours > 0.0 and arial > 0.0):
        return None
    return math.floor(ours / arial * 1000.0 + 0.5) / 1000.0


def fitted(wf: float | None, line: str, bold: bool) -> float | None:
    """A Standard text's 41: its width factor times Arial's fit, to six places (its own when there is no fit)."""
    f = arial_fit(line, bold)
    if f is None:
        return wf
    return math.floor((1.0 if wf is None else float(wf)) * f * 1e6 + 0.5) / 1e6


def standard(e: dict, names: tuple) -> bool:
    """A text in no style of the project and no typeface of its own: Standard, KentOS's own face."""
    text, _, _ = names
    return e.get("textStyle") not in text and not e.get("font")


# ── Capital heights (docs/adr/8046 §3) ────────────────────────────────
# Other programs read a TrueType text's height (40) as its capitals' height (their sources: docs/adr/8046 §3);
# KentOS draws it as the em. A text's
# 40 is its height times the capital share of the face its STYLE's file names: Arial's H, 1466 of 2048 (arial*.ttf,
# Standard's); Courier New's 1170, 1171 (couri), 1213 (courbd, courbi); KentOS's own faces their files' OS/2 capital
# height (Barlow 700/1000, Arimo 1409/2048 …); a face KentOS has not got Barlow's; an SHX file, or a name without a
# TrueType extension, none. A TEXT's or an ATTDEF's KENTOS data holds its own height ("height") exactly when the
# capitals' height divided back does not give it.
OWN_FILES = {"barlow": "Barlow-400.ttf", "arimo": "Arimo-400.ttf", "overpass": "Overpass-400.ttf",
             "quicksand": "Quicksand-400.ttf", "architectsdaughter": "ArchitectsDaughter-400.ttf",
             "courierprime": "CourierPrime-400.ttf", "ibmplexmono": "IBMPlexMono-400.ttf",
             "plexmono": "IBMPlexMono-400.ttf"}
STYLE_WORDS = ("", "regular", "normal", "roman", "medium", "semibold", "bold", "italic", "oblique", "bolditalic",
               "boldoblique", "400", "500", "600", "700", "400italic", "bd", "bi", "i", "b", "z")


def os2_cap(path: Path) -> float:
    """A TrueType face's OS/2 capital height over its units per em, read with the standard library."""
    data = path.read_bytes()
    tables = {}
    for i in range(struct.unpack(">H", data[4:6])[0]):
        tag, _, off, _ = struct.unpack(">4sIII", data[12 + 16 * i : 28 + 16 * i])
        tables[tag.decode("latin-1")] = off
    upm = struct.unpack(">H", data[tables["head"] + 18 : tables["head"] + 20])[0]
    o = tables["OS/2"]
    return struct.unpack(">h", data[o + 88 : o + 90])[0] / upm


def cap_share(file: str) -> float | None:
    """The capital share of the face a STYLE's file names, by the rule above."""
    leaf = file.strip().lower().replace("\\", "/").rsplit("/", 1)[-1]
    if not leaf.endswith((".ttf", ".otf", ".ttc")):
        return None
    stem = leaf.rsplit(".", 1)[0]
    compact = "".join(c for c in stem if c.isascii() and c.isalnum())
    styled = lambda fam: compact.startswith(fam) and compact[len(fam):] in STYLE_WORDS
    for fam, face in OWN_FILES.items():
        if styled(fam):
            return os2_cap(FACES / face)
    if styled("liberationsans"):
        return os2_cap(FACES / OWN_FILES["arimo"])
    if styled("arial") or styled("helvetica"):
        return 1466 / 2048
    if styled("couriernew") or styled("courier") or styled("cour"):
        if stem in ("courbd", "courbi") or "bold" in compact:
            return 1213 / 2048
        return 1171 / 2048 if (stem == "couri" or "italic" in compact) else 1170 / 2048
    return os2_cap(FACES / OWN_FILES["barlow"])


def text_share(e: dict, names: tuple, spec: dict) -> float | None:
    """The capital share of the record a text (or a table's cells, a curved text's letters) names: its style's, its
    typeface's, else Standard's (arial.ttf)."""
    text, _, _ = names
    if e.get("textStyle") in text:
        s = next(x for x in spec.get("textStyles", []) if x["id"] == e["textStyle"])
        return cap_share(s.get("fontFile") or font_file(s["font"], bool(s.get("bold")), bool(s.get("italic"))))
    if e.get("font"):
        return cap_share(font_file(e["font"], bool(e.get("bold")), bool(e.get("italic"))))
    return cap_share("arial.ttf")


def capitals(h: float, share: float | None) -> float:
    """A height (an em) as the file's 40: times the capital share."""
    return h if share is None else h * share


def kept_height(h: float, share: float | None) -> float | None:
    """The height KentOS's data keeps: when the file's divided back does not give it exactly."""
    return None if share is None or (h * share) / share == h else h


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


def kentos_pairs(e: list[tuple[int, str]], name: str) -> dict[str, str]:
    """KentOS's items of two values on an entity (its "attr" or "tagname" items): first -> second."""
    out: dict[str, str] = {}
    at = next((i for i, g in enumerate(e) if g == (1001, "KENTOS")), None)
    if at is None:
        return out
    i = at + 1
    while i < len(e):
        if e[i] == (1002, "{") and i + 1 < len(e) and e[i + 1] == (1000, name):
            k = i + 2
            values = []
            while k < len(e) and e[k] != (1002, "}"):
                values.append(e[k][1])
                k += 1
            ensure(len(values) == 2, f"a {name} item is two values: {values}")
            out[values[0]] = values[1]
            i = k + 1
        else:
            i += 1
    return out


def kentos_attrs(e: list[tuple[int, str]]) -> dict[str, str]:
    """KentOS's attributes on an entity (its "attr" items): key -> value."""
    return kentos_pairs(e, "attr")


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
    """A text written as an MTEXT (docs/adr/0182 §5): it has a line break, a box, a line spacing or letter formats;
    not one along a curve (docs/adr/0196 §5)."""
    return e["kind"] == "text" and "path" not in e and ("\n" in e["text"] or any(k in e for k in ("boxWidth", "lineSpacing", "runs")))


def is_curved(e: dict) -> bool:
    """A text along a curve (docs/adr/0196 §5): an anonymous block of its letters and its INSERT."""
    return e["kind"] == "text" and "path" in e


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


def spacing_written(spacing: float | None, share: float | None) -> float | None:
    """An MTEXT's 44 (docs/adr/8046 §3): other programs' lines are 5/3 × 40 × 44 apart, KentOS's 5/3 × em × spacing; with a
    share the spacing (1 when none) over it, within 0.25 to 4."""
    if share is None:
        return spacing
    return min(max((1.0 if spacing is None else spacing) / share, 0.25), 4.0)


def check_paragraph(o: list[tuple[int, str]], e: dict, where: str, styled: tuple, spec: dict) -> None:
    """A multi-line text's MTEXT (docs/adr/0182 §5); its 40 and 44 in its capitals' height (docs/adr/8046 §3)."""
    r = math.radians(e["rotation"])
    h = e["height"]
    share = text_share(e, styled, spec)
    # Its attachment point lowered by its share over the first baseline (1 top, ½ middle, −0.2 bottom) of its em less
    # its capitals' height, so another program's baseline is KentOS's (docs/adr/8046 §3); KentOS's own in its data.
    def lowered(at: tuple[float, float], up: float) -> tuple[tuple[float, float], float]:
        d = 0.0 if share is None else up * (1.0 - share) * h
        return (at[0] + math.sin(r) * d, at[1] - math.cos(r) * d), d
    kept = kentos(o).get("at")
    if e.get("align") in ATTACH:
        n = ATTACH[e["align"]]
        ensure(group(o, 71) == str(n), f"{where}: attached as aligned")
        want, d = lowered(xy(e["p"]), 1.0 if n <= 3 else (0.5 if n <= 6 else -0.2))
        got = point(o, 10)
        ensure(abs(got[0] - want[0]) <= 1e-9 and abs(got[1] - want[1]) <= 1e-9, f"{where}: at its point lowered by {d}, {want}")
        ensure((kept is None) if d == 0.0 else (kept is not None and [float(x) for x in kept] == list(xy(e["p"]))), f"{where}: KentOS's own point in its data")
    else:
        ensure(group(o, 71) == str(BASELINE_TOP[e.get("align")]), f"{where}: a baseline's as the top of its side")
        if e.get("align") is None:
            # The box's top is a height over the first baseline.
            top = (e["p"]["x"] - math.sin(r) * h, e["p"]["y"] + math.cos(r) * h)
            want, d = lowered(top, 1.0)
            got = point(o, 10)
            ensure(abs(got[0] - want[0]) <= 1e-9 and abs(got[1] - want[1]) <= 1e-9, f"{where}: at its box's top left {top} lowered by {d}, not {got}")
            ensure((kept is None) if d == 0.0 else (kept is not None and abs(float(kept[0]) - top[0]) <= 1e-9 and abs(float(kept[1]) - top[1]) <= 1e-9), f"{where}: KentOS's own point in its data")
    ensure(float(group(o, 40)) == capitals(h, share), f"{where}: its capitals' height")
    keep = kept_height(h, share)
    got = kentos(o).get("height")
    ensure((got is None) if keep is None else (got is not None and float(got[0]) == keep), f"{where}: its own height {keep} in KentOS's data")
    ensure(float(group(o, 41)) == float(e.get("boxWidth") or 0.0), f"{where}: its box's width (0 without one)")
    ensure(group(o, 72) == "1", f"{where}: left to right")
    factor = spacing_written(e.get("lineSpacing"), share)
    if factor is not None:
        ensure(group(o, 73) == "1" and float(group(o, 44)) == factor, f"{where}: its line spacing {factor} (44)")
        back = factor * share if share else factor
        back = None if abs(back - 1.0) <= 1e-9 else back
        exact = back == e.get("lineSpacing") or (back is None and e.get("lineSpacing") is None)
        got = kentos(o).get("spacing")
        want = None if exact else (e.get("lineSpacing") or 1.0)
        ensure((got is None) if want is None else (got is not None and float(got[0]) == want), f"{where}: its own line spacing {want} in KentOS's data")
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


CURVE_FLOATS = ("rotation", "height", "widthFactor", "oblique")


def curve_data(e: dict) -> str:
    """A curved text's own fields as KentOS's data holds them: the contract's JSON, keys in order, numbers as floats."""
    t = {k: v for k, v in e.items() if k not in TABLE_BASE}
    for k in CURVE_FLOATS:
        if k in t:
            t[k] = float(t[k])
    t["p"] = {"x": float(t["p"]["x"]), "y": float(t["p"]["y"])}
    path = {"pts": [{"x": float(q["x"]), "y": float(q["y"])} for q in t["path"]["pts"]]}
    if "bulges" in t["path"]:
        path["bulges"] = [float(b) for b in t["path"]["bulges"]]
    t["path"] = path
    return json.dumps(t, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def check_curved(o: list[tuple[int, str]], e: dict, blocks: list, by_name: dict, layers: dict, styled: tuple, where: str, n: int,
                 spec: dict) -> None:
    """A text along a curve (docs/adr/0196 §5): an INSERT of the anonymous block *Un at its point, turned as its frame,
    KentOS's data its own fields; the block in the text's frame (its point the origin, unturned): a TEXT a letter that
    is not a space, in order, where the layout of text_along_cases.py puts it (measured in the text's typeface, else
    Arimo, the file's Arial), turned as the curve there, its height and width factor, on 0 in BYBLOCK."""
    import text_along_cases as along

    name = f"*U{n}"
    ensure(group(o, 2) == name and group(o, 330) == "17" and group(o, 8) == layers[e["layerId"]], f"{where}: {name}'s INSERT in model space, on its layer")
    ensure((float(group(o, 10)), float(group(o, 20))) == (float(e["p"]["x"]), float(e["p"]["y"])), f"{where}: at its point")
    ensure((group(o, 50) is None) if e["rotation"] == 0 else float(group(o, 50)) == float(e["rotation"]), f"{where}: its turn")
    if e.get("color"):
        ensure(group(o, 420) == str(int(e["color"][1:], 16)), f"{where}: its true colour")
    data = kentos(o).get("along")
    ensure(data is not None and item_text(data) == curve_data(e), f"{where}: KentOS's data is its fields")
    ensure(kentos_attrs(o) == e["attrs"], f"{where}: its attributes")
    k = by_name[name]
    end = next(i for i in range(k, len(blocks)) if blocks[i][0] == (0, "ENDBLK"))
    head, objects = blocks[k], blocks[k + 1 : end]
    ensure(group(head, 70) == "1" and (float(group(head, 10)), float(group(head, 20))) == (0.0, 0.0), f"{where}: anonymous, its base at the origin")
    ensure(all(x[0] == (0, "TEXT") and group(x, 8) == "0" and group(x, 62) == "0" for x in objects), f"{where}: its block's TEXTs on 0 in BYBLOCK")
    local = {**e, "p": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "drawingFont": "arimo"}
    laid, _ = along.letters(local)
    shown = [(c, l) for c, l in zip(e["text"], laid) if not c.isspace()]
    ensure(len(objects) == len(shown), f"{where}: a TEXT a letter ({len(shown)})")
    for i, ((c, l), x) in enumerate(zip(shown, objects)):
        w = f"{where} › {i + 1}. harf {c!r}"
        ensure(group(x, 1) == table_text(c), f"{w}: its letter")
        ensure(close(float(group(x, 10)), l["at"][0]) and close(float(group(x, 20)), l["at"][1]), f"{w}: where the layout puts it")
        turn = float(group(x, 50) or 0.0)
        ensure(abs((turn - l["turn"] + 180.0) % 360.0 - 180.0) <= 1e-9, f"{w}: turned as the curve ({l['turn']})")
        ensure(float(group(x, 40)) == capitals(float(e["height"]), text_share(e, styled, spec)), f"{w}: its capitals' height")
        wf = fitted(e.get("widthFactor"), table_text(c), bool(e.get("bold"))) if standard(e, styled) else e.get("widthFactor")
        ensure((group(x, 41) is None) if wf is None else float(group(x, 41)) == float(wf), f"{w}: its width factor {wf}")
        check_face(x, e, styled, False, w, letter=True)


def check_table(o: list[tuple[int, str]], e: dict, blocks: list, by_name: dict, layers: dict, where: str, n: int,
                styled: tuple, spec: dict) -> None:
    """A table (docs/adr/0184 §7): an INSERT of the anonymous block *Un at its corner, turned as it is, KentOS's data
    its own fields; the block in its own axes (the corner at the origin, y down the page as −y): a LINE for every run of
    drawn edges (the outline's only without a frame band; none without lines), the band as four SOLIDs, a TEXT for every
    cell with words (a merged range's in its whole box), left, centred or right at half a height in, its baseline 0.35
    of a height under the middle, on 0 in BYBLOCK; in KentOS's own face its 41 fitted to Arial, bold as KentOS draws
    the cell (its style's, else a heading row's or the face's; docs/adr/8046 §2)."""
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
            bold = next((bool(c["bold"]) for c in reversed(e.get("styles") or [])
                         if c.get("bold") is not None
                         and holds({"rows": 1, "cols": 1, **c}, i, j)), bool(e.get("header") and i == 0) or bool(e.get("bold")))
            want.append((table_text(words), align, (x, -((y0 + y1) / 2.0 + 0.35 * h)), bold))
    ensure(len(texts) == len(want), f"{where}: {len(want)} TEXTs")
    share = text_share(e, styled, spec)
    for t, (words, align, at, bold) in zip(texts, want):
        ensure(group(t, 1) == words and float(group(t, 40)) == capitals(h, share), f"{where}: “{words}” at its capitals' height")
        wf = fitted(None, words, bold) if standard(e, styled) else None
        ensure((group(t, 41) is None) if wf is None else float(group(t, 41)) == wf, f"{where}: “{words}” width factor {wf}")
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


# The DXF arrow block of each arrowhead (docs/adr/0205 §7); the filled one is Standard's, none has none.
ARROW_BLOCK = {
    "closed": "_ClosedBlank", "open": "_Open", "open30": "_Open30", "open90": "_Open90", "dot": "_Dot",
    "dotSmall": "_DotSmall", "dotBlank": "_DotBlank", "oblique": "_Oblique", "archTick": "_ArchTick",
    "boxFilled": "_BoxFilled", "boxBlank": "_BoxBlank", "datumFilled": "_DatumFilled",
}


def arrow_shape(arrow: str) -> list[tuple]:
    """An arrow block's objects as other programs draw the arrowhead, one long, its tip at the origin, its line along −x:
    ("solid", corners), ("donut", centre, radius), ("polyline", points, closed), ("circle", centre, radius)."""
    t = 1.0 / 6.0
    w = 1.0 / 16.0 / math.sqrt(2.0)
    s = math.tan(math.pi / 12.0)
    return {
        "closed": [("polyline", [(0.0, 0.0), (-1.0, -t), (-1.0, t)], True)],
        "open": [("polyline", [(-1.0, -t), (0.0, 0.0), (-1.0, t)], False)],
        "open30": [("polyline", [(-1.0, -s), (0.0, 0.0), (-1.0, s)], False)],
        "open90": [("polyline", [(-0.5, -0.5), (0.0, 0.0), (-0.5, 0.5)], False)],
        "dot": [("donut", (0.0, 0.0), 0.25)],
        "dotSmall": [("donut", (0.0, 0.0), 0.125)],
        "dotBlank": [("circle", (0.0, 0.0), 0.25)],
        "oblique": [("polyline", [(0.5, 0.5), (-0.5, -0.5)], False)],
        "archTick": [("solid", [(0.5 - w, 0.5 + w), (-0.5 - w, -0.5 + w), (-0.5 + w, -0.5 - w), (0.5 + w, 0.5 - w)])],
        "boxFilled": [("solid", [(0.25, 0.25), (-0.25, 0.25), (-0.25, -0.25), (0.25, -0.25)])],
        "boxBlank": [("polyline", [(0.25, 0.25), (-0.25, 0.25), (-0.25, -0.25), (0.25, -0.25)], True)],
        "datumFilled": [("solid", [(0.0, 0.5), (-1.0, 0.0), (0.0, -0.5)])],
    }[arrow]


def check_arrow_block(b: list[list[tuple[int, str]]], arrow: str, record: str, where: str) -> None:
    """An arrow block's BLOCK, objects and ENDBLK: not anonymous, on 0, its objects BYBLOCK, as `arrow_shape`."""
    head, *objects, end = b
    name = ARROW_BLOCK[arrow]
    ensure(group(head, 2) == name and group(head, 70) == "0" and group(head, 330) == record, f"{where}: {name}'s BLOCK, not anonymous")
    ensure(float(group(head, 10)) == 0.0 and float(group(head, 20)) == 0.0, f"{where}: {name} based at the origin")
    ensure(end[0] == (0, "ENDBLK") and group(end, 330) == record, f"{where}: {name}'s ENDBLK")
    want = arrow_shape(arrow)
    ensure(len(objects) == len(want), f"{where}: {name}'s {len(want)} objects")
    near = lambda a, b: abs(a - b) <= 1e-12
    for o, (kind, *rest) in zip(objects, want):
        ensure(group(o, 330) == record and group(o, 8) == "0" and group(o, 62) == "0" and group(o, 370) == "-2", f"{where}: {name}'s objects on 0, BYBLOCK")
        if kind == "solid":
            corners = rest[0]
            got = [(float(group(o, c)), float(group(o, c + 10))) for c in (10, 11, 12, 13)]
            # SOLID runs 1 2 4 3 round a four-cornered area; a triangle repeats its last corner.
            order = [corners[0], corners[1], corners[3], corners[2]] if len(corners) == 4 else [*corners, corners[2]]
            ensure(o[0] == (0, "SOLID") and all(near(a[0], b[0]) and near(a[1], b[1]) for a, b in zip(got, order)), f"{where}: {name}'s SOLID {order}")
        elif kind == "donut":
            (cx, cy), r = rest
            xs = [float(v) for c, v in o if c == 10]
            ys = [float(v) for c, v in o if c == 20]
            ensure(o[0] == (0, "LWPOLYLINE") and group(o, 70) == "1" and float(group(o, 43)) == r, f"{where}: {name}'s donut as wide as its radius")
            ensure(xs == [cx - r / 2, cx + r / 2] and ys == [cy, cy] and [v for c, v in o if c == 42] == ["1.0", "1.0"], f"{where}: {name}'s two half circles")
        elif kind == "circle":
            (cx, cy), r = rest
            ensure(o[0] == (0, "CIRCLE") and float(group(o, 10)) == cx and float(group(o, 20)) == cy and float(group(o, 40)) == r, f"{where}: {name}'s circle")
        else:
            pts, closed = rest
            got = [(float(a[1]), float(b[1])) for a, b in zip(o, o[1:]) if a[0] == 10 and b[0] == 20]
            ensure(o[0] == (0, "LWPOLYLINE") and int(group(o, 70)) & 127 == (1 if closed else 0), f"{where}: {name}'s polyline")
            ensure(len(got) == len(pts) and all(near(a[0], b[0]) and near(a[1], b[1]) for a, b in zip(got, pts)), f"{where}: {name}'s points {pts}")


def check_leader(o: list[tuple[int, str]], m: list[tuple[int, str]] | None, e: dict, where: str, arrows: dict[str, str]) -> None:
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
    # Its note's height as its capitals' in Standard's Arial (docs/adr/8046 §3); its own in KentOS's data when needed.
    arial = 1466 / 2048
    ensure(float(group(o, 40)) == capitals(h, arial), f"{where}: its capitals' height")
    keep = kept_height(h, arial)
    got = kentos(o).get("height")
    ensure((got is None) if keep is None else (got is not None and float(got[0]) == keep), f"{where}: its own height {keep} in KentOS's data")
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
    # Its arrowhead's length and its DXF arrow block (docs/adr/0205 §7).
    size = e.get("arrowSize")
    own = dstyle(o)
    ensure(41 in own and float(own[41]) == (size if size is not None else 1.0) * h, f"{where}: its arrowhead's length (DSTYLE 41)")
    block = ARROW_BLOCK.get(arrow)
    ensure(own.get(341) == (arrows[block] if block else None), f"{where}: the arrow block {block} (DSTYLE 341)")
    ensure(set(own) <= {41, 341}, f"{where}: no other change to Standard")
    ensure("arrow" not in items, f"{where}: no arrow item (the block names it)")
    exact_size = size is not None and (size * h) / h != size
    ensure(("arrowsize" in items) == exact_size, f"{where}: KentOS's arrow size exactly when DIMASZ over its height does not give it back")
    if exact_size:
        ensure(float(items["arrowsize"][0]) == size, f"{where}: KentOS's arrow size exactly")
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
    # Its middle lowered by half its em less half its capitals' height (docs/adr/8046 §3).
    d = 0.5 * (1.0 - arial) * h
    at = (at[0] + math.sin(r) * d, at[1] - math.cos(r) * d)
    ensure(abs(float(group(m, 10)) - at[0]) <= 1e-8 and abs(float(group(m, 20)) - at[1]) <= 1e-8, f"{where}: the note 2.5 heights past the last vertex, lowered ({at})")
    ensure(float(group(m, 40)) == capitals(h, arial), f"{where}: the note as high as the leader, its capitals'")
    got = kentos(m).get("height")
    ensure((got is None) if keep is None else (got is not None and float(got[0]) == keep), f"{where}: the note's own height {keep} in KentOS's data")
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
    follows = e.get("byLayer", [])
    if e.get("lineType") is None and "lineType" not in follows:
        ensure(group(o, 6) == "ByBlock", f"{where}: BYBLOCK line type (6 ByBlock)")
    if e.get("transparency") is None and "transparency" not in follows:
        ensure(group(o, 440) == str(0x01000000), f"{where}: BYBLOCK transparency (440 16777216)")


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
    # Lowered so another program's baseline is KentOS's (docs/adr/8046 §3): its row's share of its em less its
    # capitals' height (the file's 40).
    row_up = ROWS[next(r for r in ROWS if align.startswith(r))]
    share = t.get("share")
    d = 0.0 if share is None else row_up * (1.0 - share) * float(t["height"])
    turn = math.radians(float(t["rotation"]))
    want = (p[0] + math.sin(turn) * d, p[1] - math.cos(turn) * d)
    if exact and d == 0.0:
        ensure(x11 == p[0] and y11 == p[1], f"{where}: stands on its point (11) exactly")
    else:
        ensure(close(x11, want[0]) and close(y11, want[1]), f"{where}: stands at its point lowered by {d} (11), {want}")
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
    file_height = capitals(height, share)
    ensure(abs(across - ROWS[row] * file_height) <= 1e-8, f"{where}: starts {ROWS[row]} of the file's height under 11, not {across / file_height}")
    if COLUMNS[column] == 0.0:
        ensure(abs(run) <= 1e-8, f"{where}: starts on 11's side")
    else:
        widths.setdefault((shown, height, wf or 1.0), []).append((run / COLUMNS[column], where))
        # Its width as a program that does not justify draws it from 10: Arial's (Arimo's advances) at its height,
        # widened by the file's 41 — a fitted text's is KentOS's line (docs/adr/8046 §2).
        import text_along_cases as along

        arial = sum(along.advance("arimo", c, False) for c in shown) / 1000.0 * height * (wf or 1.0)
        ensure(abs(run / COLUMNS[column] - arial) <= 1e-8 * max(1.0, arial),
               f"{where}: starts its Arial width {arial} back, not {run / COLUMNS[column]}")


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


def placed_frame(e: dict, degrees: float, align: str | None) -> tuple[float, str | None]:
    """A text's turn and alignment under the insert's similarity; a mirrored insert's readable (docs/adr/0292)."""
    s = float(e["scale"])
    flip = -1.0 if e.get("mirror") else 1.0
    c, n = math.cos(e["rotation"]), math.sin(e["rotation"])
    turn, align, _ = text_frame(degrees, align, [s * c, s * n, -s * n * flip, s * c * flip, 0.0, 0.0])
    return turn, align


def extra_attributes(e: dict, b: dict) -> list[tuple[str, str, str]]:
    """An insert's attributes no definition of its block holds that go out as invisible ATTRIBs (docs/adr/3980), by
    name in code point order as KentOS keeps them: each named, its value one line (no control character); its tag its
    name, white space and control characters "_", a tag taken before (a definition's or an earlier one's, but for case)
    "_2", "_3" … (docs/adr/4380). Each one's name, tag and value."""
    own = {a["tag"] for a in b.get("attributes") or []}
    taken = {t.upper() for t in dxf_tags(b)}
    out = []
    for name, value in sorted((e.get("attrs") or {}).items()):
        if name in own or not name or any(control(c) for c in value):
            continue
        base = "".join("_" if (c.isspace() or control(c)) else c for c in name)
        tag, k = base, 2
        while tag.upper() in taken:
            tag = f"{base}_{k}"
            k += 1
        taken.add(tag.upper())
        out.append((name, tag, value))
    return out


def check_attributes(o: list[list[tuple[int, str]]], e: dict, b: dict, layer: str, own: str, where: str, widths: dict,
                     per_metre: float) -> None:
    """An insert's ATTRIBs and SEQEND (`o`: the entities after it; `own`: its handle): its definitions', then its other
    attributes' (invisible), then SEQEND."""
    attributes = b.get("attributes") or []
    tags = dxf_tags(b)
    extra = extra_attributes(e, b)
    n = len(attributes) + len(extra)
    ensure(len(o) > n and o[n][0] == (0, "SEQEND"), f"{where}: {len(attributes)} ATTRIB and {len(extra)} invisible, then SEQEND")
    ensure(group(o[n], 330) == own, f"{where}: its SEQEND is its own")
    for k, (a, tag) in enumerate(zip(attributes, tags)):
        r = o[k]
        w = f"{where} › {a['tag']}"
        ensure(r[0] == (0, "ATTRIB"), f"{w}: an ATTRIB")
        ensure(group(r, 330) == own, f"{w}: owned by its insert")
        ensure(group(r, 8) == layer, f"{w}: on the insert's layer")
        ensure(group(r, 2) == tag and group(r, 70) == "0", f"{w}: tag {tag!r}, shown (70 0)")
        value = (e.get("attrs") or {}).get(a["tag"]) or a.get("value") or ""
        ensure(group(r, 1) == value, f"{w}: shows {value!r}, not {group(r, 1)!r}")
        x, y = placed(e, b["base"], a["p"])
        # Its height times the scale, as its capitals' in Standard's Arial (docs/adr/8046 §3).
        ensure(close(float(group(r, 40)), a["height"] * float(e["scale"]) * (1466 / 2048)), f"{w}: its height times the scale, Arial's capitals")
        turn, align = placed_frame(e, a["rotation"], a.get("align"))
        got = float(group(r, 50) or 0.0)
        ensure(close(got, turn) or (turn == 0.0 and group(r, 50) is None), f"{w}: turned {turn}°, not {got}°")
        # An attribute names no style: Standard, fitted to Arial (docs/adr/8046 §2).
        wf = fitted(a.get("widthFactor"), value, False)
        shown_at = {**a, "height": a["height"] * float(e["scale"]), "rotation": got, "align": align, "widthFactor": wf,
                    "share": 1466 / 2048}
        if align is None:
            ensure(close(float(group(r, 10)), x) and close(float(group(r, 20)), y), f"{w}: placed at {x}, {y}")
            ensure(group(r, 11) is None, f"{w}: not justified")
            ensure((group(r, 41) is None) if wf is None else float(group(r, 41)) == wf, f"{w}: width factor {wf}")
        else:
            check_place(r, (x, y), False, shown_at, "AcDbAttribute", 74, value, widths, w)
    for k, (name, tag, value) in enumerate(extra):
        r = o[len(attributes) + k]
        w = f"{where} › {name}"
        ensure(r[0] == (0, "ATTRIB"), f"{w}: an ATTRIB")
        ensure(group(r, 330) == own and group(r, 8) == layer, f"{w}: owned by its insert, on its layer")
        ensure(group(r, 2) == tag and group(r, 1) == value, f"{w}: tag {tag!r} showing {value!r}")
        ensure(group(r, 70) == "1", f"{w}: invisible (70 1)")
        ensure(close(float(group(r, 10)), e["p"]["x"]) and close(float(group(r, 20)), e["p"]["y"]), f"{w}: at the insert's point")
        ensure(close(float(group(r, 40)), 0.0025 * per_metre), f"{w}: 2.5 mm high")
        ensure(all(c not in (11, 41, 50, 72, 74) for c, _ in r), f"{w}: neither justified, turned nor widened")


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
             "prefix", "suffix", "font", "dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight",
             "extLineType", "textColor")
# DXF's line weights, hundredths of a mm (docs/adr/0139): a weight is written as the nearest, the first of two.
DXF_WEIGHTS = [0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200, 211]
# The LTYPE records of KentOS's dashed line types.
LTYPE_NAMES = {"dashed": "DASHED", "dashdot": "DASHDOT", "dotted": "DOT"}


def nearest_weight(mm: float) -> int:
    return min(DXF_WEIGHTS, key=lambda w: abs(w - mm * 100.0))


def ltype_handles(p: list[tuple[int, str]]) -> dict[str, str]:
    """The LTYPE records' handles by name."""
    return {group(r, 2): group(r, 5) for r in split(section(p, "TABLES")) if r[0] == (0, "LTYPE")}
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
        file = s.get("fontFile", font_file(s["font"], bool(s.get("bold")), bool(s.get("italic"))))
        height = capitals(length(s["height"]), cap_share(file)) if s.get("height") else 0.0
        ensure(close(float(group(r, 40)), height, 1e-12), f"{w}: its fixed height {height}, its capitals' in drawing units (40)")
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
        # Its value's face: the project's text style it names (DIMTXSTY), else Standard's Arial (docs/adr/8046 §3).
        named = next((t for t in spec.get("textStyles", []) if s.get("textStyle") and t["name"].lower() == s["textStyle"].lower()), None)
        share = (cap_share(named.get("fontFile") or font_file(named["font"], bool(named.get("bold")), bool(named.get("italic"))))
                 if named else 1466 / 2048)
        check_vars({c: v for c, v in r if c not in (1001, 1000, 1002)}, look, h, s.get("decimals", 2), per_metre, w, ltype_handles(p), share)
        flags_at = r.index((70, "0"))
        post = dimpost(look)
        ensure(post is None or r[flags_at + 1] == (3, post), f"{w}: DIMPOST {post!r} after the flags")
        ensure(json.loads(item_text(kentos(r)["look"])) == s, f"{w}: the style itself in KentOS's data")
    return [f"{len(spec.get('textStyles', []))} yazı stili, {len(synthetic)} yazı tipi kaydı, {len(given)} ölçü stili"] if (given or spec.get("textStyles") or synthetic) else []


def dimpost(look: dict) -> str | None:
    if look.get("prefix") is None and look.get("suffix") is None:
        return None
    return f"{look.get('prefix', '')}<>{look.get('suffix', '')}"


def check_vars(v: dict, look: dict, h: float, decimals: int, per_metre: float, where: str, ltypes: dict[str, str],
               share: float | None = None) -> None:
    """A look's variables (a DIMSTYLE record's groups or a DIMENSION's DSTYLE overrides, code -> value) at height h; the
    value's height (DIMTXT) its capitals' in a face of capital share `share` (docs/adr/8046 §3), the sizes lengths of h."""
    tick = "arrow" not in look
    size = look.get("arrowSize", 0.6 if tick else 1.0) * h
    real = lambda code: float(v[code]) if code in v else None
    ensure(close(real(140), capitals(h, share), 1e-12), f"{where}: its value's capitals' height (140)")
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
    # Its lines (docs/adr/0205 §6): the colours it names as ACI indexes (DIMCLRD, DIMCLRE, DIMCLRT), its weights as
    # DXF's nearest (DIMLWD, DIMLWE), its types by their LTYPE records (DIMLTYPE; DIMLTEX1 and 2 the extension lines').
    for code, key in ((176, "dimLineColor"), (177, "extColor"), (178, "textColor")):
        if look.get(key):
            ensure(v.get(code) is not None and 1 <= int(v[code]) <= 255, f"{where}: {key} an ACI index ({code})")
        else:
            ensure(v.get(code) in (None, "0"), f"{where}: no {key} of its own ({code})")
    for code, key in ((371, "dimLineWeight"), (372, "extWeight")):
        if look.get(key) is not None:
            ensure(v.get(code) == str(nearest_weight(look[key])), f"{where}: {key} as DXF's nearest ({code})")
        else:
            ensure(v.get(code) in (None, "-2"), f"{where}: no {key} of its own ({code})")
    for code, key in ((345, "dimLineType"), (346, "extLineType"), (347, "extLineType")):
        t = look.get(key)
        ensure(v.get(code) == (ltypes[LTYPE_NAMES[t]] if t else None), f"{where}: {key} by its LTYPE record ({code})")


def check_face(o: list[tuple[int, str]], e: dict, names: tuple, mtext: bool, where: str, letter: bool = False) -> None:
    """A TEXT's or an MTEXT's style record (7), own slant (51) and face in KentOS's data; a curved text's letter only
    its record (its face is the INSERT's data, docs/adr/0196 §5)."""
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
    if letter:
        return
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


def check_dim_look(o: list[tuple[int, str]], e: dict, names: tuple, spec: dict, blocks: list, per_metre: float, where: str,
                   ltypes: dict[str, str]) -> None:
    """A DIMENSION's style record (3), its look in DSTYLE and KentOS's data, its block's arrowheads, lines' pens and value's record."""
    _, dims, synthetic = names
    look = own(e, LOOK_KEYS)
    ensure(group(o[after(o, "AcDbDimension") :], 3) == dims.get(e.get("dimStyle"), "Standard"), f"{where}: names its style's record (3)")
    # Its value is in its typeface's record, else Standard's Arial: DIMTXT its capitals' height (docs/adr/8046 §3).
    share = cap_share(font_file(e["font"], False, False)) if e.get("font") else 1466 / 2048
    check_vars(dstyle(o), look, e["height"], spec["lengthDecimals"], per_metre, f"{where} (DSTYLE)", ltypes, share)
    got = kentos(o).get("look")
    ensure((got is None) if not look else (got is not None and json.loads(item_text(got)) == look), f"{where}: its look {look} in KentOS's data")
    name = group(o, 2)
    k = next(i for i, x in enumerate(blocks) if x[0] == (0, "BLOCK") and group(x, 2) == name)
    end = next(i for i in range(k, len(blocks)) if blocks[i][0] == (0, "ENDBLK"))
    inside = blocks[k + 1 : end]
    mtext = next(x for x in inside if x[0] == (0, "MTEXT"))
    value = synthetic[(e["font"], False, False)] if e.get("font") else "Standard"
    ensure(group(mtext, 7) == value, f"{where}: its value in its typeface's record {value!r}")
    # Its lines' pens (docs/adr/0205 §6): an aligned or linear one's extension lines (each from a measured point, its
    # offset away) the extension lines' colour (420, true; 62 BYBLOCK without one), type (6) and weight (370), its
    # other lines and arrowheads the dimension line's, its value the value's colour.
    def pen(x: list[tuple[int, str]]) -> tuple:
        return (group(x, 6), group(x, 420), group(x, 370), group(x, 62) == "0")

    def want_pen(color: str | None, weight: float | None, kind: str | None) -> tuple:
        return (LTYPE_NAMES.get(kind) if kind else None, str(int(color[1:], 16)) if color else None,
                str(nearest_weight(weight)) if weight is not None else None, not color)

    line_pen = want_pen(look.get("dimLineColor"), look.get("dimLineWeight"), look.get("dimLineType"))
    ext_pen = want_pen(look.get("extColor"), look.get("extWeight"), look.get("extLineType"))
    gap = look.get("extOffset", 0.5) * e["height"]
    straight = e.get("style") in (None, "aligned", "linear")
    for x in inside:
        if x[0] == (0, "LINE"):
            p0 = point(x, 10)
            ext = straight and any(abs(math.hypot(p0[0] - q[0], p0[1] - q[1]) - gap) <= 1e-9 * max(1.0, gap) for q in (xy(e["a"]), xy(e["b"])))
            ensure(pen(x) == (ext_pen if ext else line_pen), f"{where}: its {'extension' if ext else 'dimension'} line's pen")
        elif x[0] in ((0, "SOLID"), (0, "LWPOLYLINE"), (0, "ARC")):
            ensure(pen(x) == line_pen, f"{where}: its arrowheads and arc in the dimension line's pen")
    ensure(pen(mtext) == want_pen(look.get("textColor"), None, None), f"{where}: its value in the value's colour")
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


# KentOS's library of patterns (docs/adr/0186 §2): the predefined ones of these names other programs know.
LIBRARY = ({f"ANSI3{k}" for k in range(1, 9)} | {f"ISO{k:02d}W100" for k in range(2, 15)}
           | {"LINE", "NET", "NET3", "DASH", "DOTS", "BRICK", "CROSS"})
# The gradients' DXF names by shape and whether inverted; DXF names no inverted linear one.
GRADIENT_NAMES = {("linear", False): "LINEAR", ("cylinder", False): "CYLINDER", ("cylinder", True): "INVCYLINDER",
                  ("spherical", False): "SPHERICAL", ("spherical", True): "INVSPHERICAL"}


def turned(degrees: float, x: float, y: float) -> tuple[float, float]:
    r = math.radians(degrees)
    return (math.cos(r) * x - math.sin(r) * y, math.sin(r) * x + math.cos(r) * y)


def hatch_families(h: list[tuple[int, str]]) -> list[dict]:
    """A HATCH's pattern line families (78, then 53, 43, 44, 45, 46, 79 and its 49s each), in order."""
    k = next(i for i, g in enumerate(h) if g[0] == 78)
    out = []
    i = k + 1
    for _ in range(int(h[k][1])):
        codes = [c for c, _ in h[i : i + 6]]
        ensure(codes == [53, 43, 44, 45, 46, 79], f"a family's groups in order, not {codes}")
        a, bx, by, ox, oy = (float(v) for _, v in h[i : i + 5])
        n = int(h[i + 5][1])
        dashes = [float(v) for c, v in h[i + 6 : i + 6 + n]]
        ensure(all(c == 49 for c, _ in h[i + 6 : i + 6 + n]), "its dashes (49)")
        out.append({"angle": a, "base": (bx, by), "offset": (ox, oy), "dashes": dashes})
        i += 6 + n
    return out


def colour_value(c: str) -> int:
    """A colour's true colour as 421 holds it: a hex colour itself, the ink ACI 7's white."""
    return 0xFFFFFF if c == "ink" else int(c[1:7], 16)


def check_hatch(o: list[tuple[int, str]], e: dict, layer_colours: dict[str, str], where: str) -> None:
    h = o[after(o, "AcDbHatch") :]
    p = e["pattern"]
    kind = p["type"]
    fill = kind in ("solid", "gradient")
    name = p["name"] if kind == "pattern" else ("_USER" if kind in ("lines", "cross") else "SOLID")
    ensure(group(h, 2) == name and group(h, 70) == ("1" if fill else "0"), f"{where}: {name}, filled {fill}")
    ensure(group(h, 71) == "0", f"{where}: not associative (71 0)")
    kept = kentos(o).get("hatch")
    if kind in ("lines", "cross"):
        ensure(group(h, 76) == "0", f"{where}: user-defined (76 0)")
        angle, spacing = p["angle"], p["spacing"]
        ensure(float(group(h, 52)) == angle and float(group(h, 41)) == spacing, f"{where}: its angle and spacing")
        ensure(group(h, 77) == ("1" if kind == "cross" else "0"), f"{where}: doubled exactly when crossed")
        fams = hatch_families(h)
        ensure(len(fams) == (2 if kind == "cross" else 1), f"{where}: its families")
        for turn, f in zip((0.0, 90.0), fams):
            a = angle + turn
            want = turned(a, 0.0, spacing)
            ensure(close(f["angle"], a) and f["base"] == (0.0, 0.0) and not f["dashes"], f"{where}: a whole family at {a}°")
            ensure(close(f["offset"][0], want[0]) and close(f["offset"][1], want[1]), f"{where}: its lines {spacing} apart")
        ensure(kept is None, f"{where}: no exact pattern in KentOS's data")
    elif kind == "pattern":
        ensure(group(h, 76) == ("1" if name.upper() in LIBRARY else "2"), f"{where}: predefined exactly when the library has {name}")
        alpha, scale = p["angle"], p["scale"]
        ensure(float(group(h, 52)) == alpha and float(group(h, 41)) == scale and group(h, 77) == "0", f"{where}: its angle and scale")
        fams = hatch_families(h)
        ensure(len(fams) == len(p["lines"]), f"{where}: {len(p['lines'])} families")
        for k, (f, l) in enumerate(zip(fams, p["lines"])):
            w = f"{where} › aile {k + 1}"
            a = alpha + l["angle"]
            base = turned(alpha, l["origin"][0] * scale, l["origin"][1] * scale)
            offset = turned(a, l["offset"][0] * scale, l["offset"][1] * scale)
            ensure(close(f["angle"], a), f"{w}: its angle the pattern's and its own")
            ensure(all(close(x, y) for x, y in zip(f["base"], base)), f"{w}: its base point {base}, not {f['base']}")
            ensure(all(close(x, y) for x, y in zip(f["offset"], offset)), f"{w}: its offset {offset}, not {f['offset']}")
            want = [d * scale for d in l.get("dashes") or []]
            ensure(len(f["dashes"]) == len(want) and all(close(x, y) for x, y in zip(f["dashes"], want)), f"{w}: its dashes {want}")
    if kind == "gradient":
        g = p["gradient"]
        inverted = bool(g.get("inverted"))
        half = g["shape"] == "linear" and inverted
        ensure(group(h, 76) == "1" and group(h, 450) == "1" and group(h, 453) == "2", f"{where}: a gradient of two colours")
        ensure(group(h, 470) == GRADIENT_NAMES[(g["shape"], inverted and not half)], f"{where}: the DXF name for {g}")
        ensure(close(float(group(h, 460)), math.radians((p["angle"] + (180.0 if half else 0.0)) % 360.0)), f"{where}: its angle in radians")
        ensure(float(group(h, 461)) == 0.0, f"{where}: centred")
        at = next(i for i, g2 in enumerate(h) if g2[0] == 453)
        pairs_ = [(int(h[i + 1][1]), int(h[i + 2][1])) for i in range(at + 1, len(h)) if h[i][0] == 463]
        ensure(all(h[i + 1][0] == 63 and h[i + 2][0] == 421 for i in range(at + 1, len(h)) if h[i][0] == 463), f"{where}: each colour 463, 63, 421")
        first = e.get("color") or layer_colours[e["layerId"]]
        ensure([c for _, c in pairs_] == [colour_value(first), colour_value(g["color2"])], f"{where}: its colours {first} and {g['color2']}")
        ensure(all(1 <= a <= 255 for a, _ in pairs_), f"{where}: ACI indexes")
        if first == "ink":
            ensure(pairs_[0][0] == 7, f"{where}: the ink is ACI 7")
    if kind in ("pattern", "gradient"):
        ensure(kept is not None and json.loads(item_text(kept)) == p, f"{where}: its pattern exactly in KentOS's data")


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
    # The DXF arrow blocks (docs/adr/0205 §7): each where the first leader with its arrowhead is written.
    arrow_of: dict[str, str] = {}

    def arrow_block(e: dict) -> None:
        block = ARROW_BLOCK.get(e.get("arrow"))
        if e["kind"] == "leader" and block and block not in arrow_of:
            arrow_of[block] = e["arrow"]
            want.append(block)

    for b in order:
        for e in b["entities"]:
            if e["kind"] == "dimension":
                k += 1
                want.append(f"*D{k}")
            arrow_block(e)
        want.append(names[b["id"]])
    # The drawing's dimensions' blocks and its tables' and curved texts' (*U1, *U2 …, docs/adr/0184 §7, 0196 §5), in
    # the drawing's order.
    u = 0
    anonymous = {}
    for i, e in enumerate(spec["entities"]):
        if e["kind"] == "dimension":
            k += 1
            want.append(f"*D{k}")
        elif e["kind"] == "table" or is_curved(e):
            u += 1
            anonymous[i] = u
            want.append(f"*U{u}")
        arrow_block(e)
    ensure([group(r, 2) for r in records] == want, f"block records {want}")
    blocks = split(section(p, "BLOCKS"))
    heads = [e for e in blocks if e[0] == (0, "BLOCK")]
    ensure([group(h, 2) for h in heads] == want, f"BLOCKs {want}")
    # The arrow blocks' records' handles, and their objects.
    arrows = {name: record_of[name] for name in arrow_of}
    for name, arrow in arrow_of.items():
        i = next(k for k, b in enumerate(blocks) if b[0] == (0, "BLOCK") and group(b, 2) == name)
        end = next(k for k in range(i, len(blocks)) if blocks[k][0] == (0, "ENDBLK"))
        check_arrow_block(blocks[i : end + 1], arrow, arrows[name], f"ok bloğu {name}")
    if arrow_of:
        said.append(f"{len(arrow_of)} ok bloğu ({', '.join(arrow_of)})")
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
            arial = 1466 / 2048
            ensure(float(group(d, 40)) == capitals(a["height"], arial), f"{w}: its capitals' height in Arial")
            keep = kept_height(a["height"], arial)
            got = kentos(d).get("height")
            ensure((got is None) if keep is None else (got is not None and float(got[0]) == keep), f"{w}: its own height {keep} in KentOS's data")
            check_place(d, (a["p"]["x"], a["p"]["y"]), True, {**a, "share": arial}, "AcDbAttributeDefinition", 74, tag, widths, w)
            got = kentos(d).get("at")
            ensure(got is None or [float(x) for x in got] == [a["p"]["x"], a["p"]["y"]], f"{w}: KentOS's point is its own")
            ensure(group(d, 1) == (a.get("value") or ""), f"{w}: its default")
            ensure(group(d, 3) == (a.get("prompt") or ""), f"{w}: its prompt")
            ensure(group(d, 2) == tag and group(d, 70) == "0", f"{w}: tag {tag!r}, visible")
            ensure((group(d, 50) is None) if a["rotation"] == 0 else float(group(d, 50)) == a["rotation"], f"{w}: its turn")
            # docs/adr/4800: a tag that is not the name holds the name in its own KentOS data; any other holds none.
            record = kentos(d).get("tagname")
            want = [tag, a["tag"]] if tag != a["tag"] else None
            ensure(record == want, f"{w}: KentOS's name record {want!r}, not {record!r}")
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
                check_leader(o, objects[k + 1] if note_of(e) else None, e, where, arrows)
                if note_of(e):
                    check_look(objects[k + 1], e, layers, f"{where} › MTEXT")
            if is_paragraph(e):
                check_paragraph(o, e, where, styled, spec)
            if e["kind"] == "text":
                check_face(o, e, styled, is_paragraph(e), where)
            if e["kind"] == "dimension":
                check_dimension(o, e, e.get("text"), blocks, where)
                check_dim_look(o, e, styled, spec, blocks, per_metre, where, ltype_handles(p))
            if e["kind"] == "polygon":
                own = group(o, 5)
                for h in objects[k + 1 : k + len(written_kinds(e))]:
                    ensure(kentos(h).get("hole") == [own], f"{where}: its hole names it")
            k += len(written_kinds(e))
        said.append(f"{names[b['id']]}: {len(objects)} nesne" + (f" ve {len(attributes)} ATTDEF" if attributes else ""))
        i = end

    # The drawing's inserts, each with its ATTRIBs after it; a table's INSERT names an anonymous block (*U).
    drawn = split(section(p, "ENTITIES"))
    # A drawn polyline runs its pattern on round its corners as KentOS draws it: PLINEGEN, 70 bit 128 (docs/adr/0313 §5).
    for o in drawn:
        if o[0] == (0, "LWPOLYLINE"):
            ensure(int(group(o, 70) or "0") & 128 == 128, f"{name}: a polyline with PLINEGEN (70 bit 128)")
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
        extra = extra_attributes(e, b)
        if b.get("attributes") or extra:
            ensure(group(o, 66) == "1", f"{where}: attributes follow (66 1)")
            check_attributes(drawn[k + 1 :], e, b, layers[e["layerId"]], group(o, 5), where, widths, per_metre)
            attributed += 1
        else:
            ensure(group(o, 66) is None, f"{where}: no attributes follow")
        # The values of its block's attributes go out once, as its ATTRIBs, its other one-line ones as invisible ATTRIBs
        # (docs/adr/3980); KentOS's data holds the rest, and the names their tags do not hold (docs/adr/4380).
        defined = {a["tag"] for a in b.get("attributes") or []}
        sent = {name for name, _, _ in extra}
        others = {k: v for k, v in e["attrs"].items() if k not in defined and k not in sent}
        ensure(kentos_attrs(o) == others, f"{where}: KentOS's data holds its other attributes {others}")
        renamed = {tag: name for name, tag, _ in extra if tag != name}
        ensure(kentos_pairs(o, "tagname") == renamed, f"{where}: KentOS's data names the tags {renamed}")
    if attributed:
        said.append(f"{attributed} yerleştirmenin ATTRIB'leri")

    # The drawing's tables (docs/adr/0184 §7), in the input's order.
    tables = [(i, e) for i, e in enumerate(spec["entities"]) if e["kind"] == "table"]
    written = [e for e in drawn if e[0] == (0, "INSERT") and (group(e, 2) or "").startswith("*U") and "table" in kentos(e)]
    ensure(len(written) == len(tables), f"{len(tables)} tables")
    by_name = {group(b, 2): k for k, b in enumerate(blocks) if b[0] == (0, "BLOCK")}
    for n, ((i, e), o) in enumerate(zip(tables, written)):
        check_table(o, e, blocks, by_name, layers, f"tablo {n + 1}", anonymous[i], styled, spec)
    tables = [e for _, e in tables]
    if tables:
        said.append(f"{len(tables)} tablo ({sum(1 for e in tables if e.get('frame') and e.get('grid') != 'none')} kalın çerçeveli)")

    # The drawing's texts along a curve (docs/adr/0196 §5), in the input's order.
    curved = [(i, e) for i, e in enumerate(spec["entities"]) if is_curved(e)]
    written = [e for e in drawn if e[0] == (0, "INSERT") and (group(e, 2) or "").startswith("*U") and "along" in kentos(e)]
    ensure(len(written) == len(curved), f"{len(curved)} curved texts")
    for n, ((i, e), o) in enumerate(zip(curved, written)):
        check_curved(o, e, blocks, by_name, layers, styled, f"eğri boyunca yazı {n + 1}", anonymous[i], spec)
    if curved:
        said.append(f"{len(curved)} eğri boyunca yazı ({sum(1 for _, e in curved if 'bulges' in e['path'])} yaylı)")

    # The drawing's texts (docs/adr/0145 §7), in the input's order.
    texts = [e for e in spec["entities"] if e["kind"] == "text" and not is_paragraph(e) and not is_curved(e)]
    written = [e for e in drawn if e[0] == (0, "TEXT")]
    ensure(len(written) == len(texts), f"{len(texts)} TEXTs")
    for n, (e, o) in enumerate(zip(texts, written)):
        where = f"yazı {n + 1} ({e['text']})"
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its layer")
        share = text_share(e, styled, spec)
        ensure(group(o, 1) == e["text"] and float(group(o, 40)) == capitals(e["height"], share), f"{where}: its words and capitals' height")
        keep = kept_height(e["height"], share)
        got = kentos(o).get("height")
        ensure((got is None) if keep is None else (got is not None and float(got[0]) == keep), f"{where}: its own height {keep} in KentOS's data")
        ensure((group(o, 50) is None) if e["rotation"] == 0 else float(group(o, 50)) == e["rotation"], f"{where}: its turn")
        fit = arial_fit(e["text"], bool(e.get("bold"))) if standard(e, styled) else None
        shown = {**e, "widthFactor": fitted(e.get("widthFactor"), e["text"], bool(e.get("bold")))} if fit else e
        shown = {**shown, "share": text_share(e, styled, spec)}
        check_place(o, (e["p"]["x"], e["p"]["y"]), True, shown, "AcDbText", 73, e["text"], widths, where)
        got = kentos(o).get("at")
        ensure(got is None or [float(x) for x in got] == [e["p"]["x"], e["p"]["y"]], f"{where}: KentOS's point is its own")
        ensure(("mask" in kentos(o)) == bool(e.get("mask")), f"{where}: KentOS's mask item exactly when masked")
        got = kentos(o).get("fit")
        ensure((got is None) if fit is None else (got is not None and float(got[0]) == fit), f"{where}: Arial's fit {fit} in KentOS's data")
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
        check_paragraph(o, e, where, styled, spec)
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
        check_leader(o, drawn[k + 1] if note_of(e) else None, e, where, arrows)
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
        check_dim_look(o, e, styled, spec, blocks, per_metre, where, ltype_handles(p))
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

    # The drawing's hatches (docs/adr/0186 §9), in the input's order.
    hatches = [e for e in spec["entities"] if e["kind"] == "hatch"]
    written = [o for o in drawn if o[0] == (0, "HATCH")]
    ensure(len(written) == len(hatches), f"{len(hatches)} HATCHes")
    layer_colours = {l["id"]: l["color"] for l in spec["layers"]}
    for n, (e, o) in enumerate(zip(hatches, written)):
        check_hatch(o, e, layer_colours, f"tarama {n + 1}")
    if any(e["pattern"]["type"] in ("pattern", "gradient") for e in hatches):
        kinds = [e["pattern"]["type"] for e in hatches]
        said.append(f"{len(hatches)} tarama ({kinds.count('pattern')} desen, {kinds.count('gradient')} degrade)")

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
