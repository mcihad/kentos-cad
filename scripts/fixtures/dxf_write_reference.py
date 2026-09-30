#!/usr/bin/env python3
"""Independent check of the DXF writer's blocks (docs/adr/0144 §5).

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
        ensure(group(head, 70) == "0", f"{b['name']}: not anonymous")
        if b.get("description"):
            one = "".join(" " if ord(c) < 32 else c for c in b["description"])
            ensure(group(head, 4) == one, f"{b['name']}: its description on one line")
        end = next(k for k in range(i, len(blocks)) if blocks[k][0] == (0, "ENDBLK"))
        objects = blocks[i + 1 : end]
        ensure(group(blocks[end], 330) == rec, f"{b['name']}: ENDBLK is its record's")
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
        said.append(f"{names[b['id']]}: {len(objects)} nesne")
        i = end

    # The drawing's inserts.
    placed = [e for e in split(section(p, "ENTITIES")) if e[0] == (0, "INSERT")]
    given = [e for e in spec["entities"] if e["kind"] == "insert" and e["block"] in names]
    ensure(len(placed) == len(given), f"{len(given)} inserts written, the unknown block's left out")
    for n, (o, e) in enumerate(zip(placed, given)):
        where = f"yerleştirme {n + 1}"
        ensure(group(o, 330) == "17", f"{where}: in model space")
        ensure(group(o, 8) == layers[e["layerId"]], f"{where}: on its layer")
        check_insert(o, e, names[e["block"]], where)
        attrs = [g for g in o if g[0] == 1000]
        for k, v in e["attrs"].items():
            ensure(any(a == (1000, k) for a in attrs) and any(a == (1000, v) for a in attrs), f"{where}: attribute {k}={v}")

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
