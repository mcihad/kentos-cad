#!/usr/bin/env python3
"""Independent check of the DXF writer's blocks (docs/adr/0144 §5) and texts (docs/adr/0145 §7).

Reads `fixtures/formats/v1/dxf-write/<name>.input.json` (the writer's input,
written by hand) and `<name>.dxf` (what the writer made of it, committed) and
checks the file against the input from the rules alone, with the standard
library and no KentOS code:

- the blocks the objects place, and those nested in them, are BLOCKs with a
  BLOCK_RECORD each, in the order of a depth-first walk from the drawing's
  inserts (a block after the ones it holds); an unused block is not written;
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
- every handle is unique and every owner names a handle of the file.

    python3 scripts/fixtures/dxf_write_reference.py --check
"""

from __future__ import annotations

import json
import math
import sys
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
    if kind == "polygon":
        return ["LWPOLYLINE"] * (1 + len(e.get("holes") or []))
    # The fixture's blocks hold only these kinds.
    return [{"line": "LINE", "circle": "CIRCLE", "point": "POINT", "insert": "INSERT"}[kind]]


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


def check(name: str) -> list[str]:
    spec = json.loads((DIR / f"{name}.input.json").read_text(encoding="utf-8"))
    p = pairs((DIR / f"{name}.dxf").read_bytes())
    layers = {l["id"]: l["name"] for l in spec["layers"]}
    order = walk(spec["blocks"], spec["entities"])
    names = dxf_names(order)
    said = []
    widths: dict = {}

    # Records and BLOCKs, in the walk's order after model and paper space.
    tables = section(p, "TABLES")
    records = [r for r in split(tables) if r[0] == (0, "BLOCK_RECORD")]
    record_of = {group(r, 2): group(r, 5) for r in records}
    want = ["*Model_Space", "*Paper_Space"] + [names[b["id"]] for b in order]
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
            if e["kind"] == "polygon":
                own = group(o, 5)
                for h in objects[k + 1 : k + len(written_kinds(e))]:
                    ensure(kentos(h).get("hole") == [own], f"{where}: its hole names it")
            k += len(written_kinds(e))
        said.append(f"{names[b['id']]}: {len(objects)} nesne" + (f" ve {len(attributes)} ATTDEF" if attributes else ""))
        i = end

    # The drawing's inserts, each with its ATTRIBs after it.
    drawn = split(section(p, "ENTITIES"))
    at = [k for k, e in enumerate(drawn) if e[0] == (0, "INSERT")]
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

    # The drawing's texts (docs/adr/0145 §7), in the input's order.
    texts = [e for e in spec["entities"] if e["kind"] == "text"]
    written = [e for e in drawn if e[0] == (0, "TEXT")]
    ensure(len(written) == len(texts), f"{len(texts)} TEXTs")
    for n, (e, o) in enumerate(zip(texts, written)):
        where = f"yazı {n + 1} ({e['text']})"
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its layer")
        ensure(group(o, 1) == e["text"] and float(group(o, 40)) == e["height"], f"{where}: its words and height")
        ensure((group(o, 50) is None) if e["rotation"] == 0 else float(group(o, 50)) == e["rotation"], f"{where}: its turn")
        check_place(o, (e["p"]["x"], e["p"]["y"]), True, e, "AcDbText", 73, e["text"], widths, where)
        ensure(("mask" in kentos(o)) == bool(e.get("mask")), f"{where}: KentOS's mask item exactly when masked")
    check_widths(widths)
    if texts:
        aligned = sum(1 for e in texts if e.get("align"))
        said.append(f"{len(texts)} yazı ({aligned} hizalı, {sum(1 for e in texts if e.get('mask'))} zeminli)")

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
