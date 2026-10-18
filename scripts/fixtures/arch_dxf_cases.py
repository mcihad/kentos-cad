#!/usr/bin/env python3
"""The architectural objects in DXF (docs/adr/0347 §2), checked without KentOS code.

Reads fixtures/formats/v1/dxf-arch/mimari.kcad with the independent KCAD reader (tools/kcad/kcad.py) and the DXF KentOS
wrote from it, mimari.dxf (`KENTOS_WRITE_DXF=1 cargo test -p kentos-headless --test headless arch_dxf_goes_round`
writes both), from DXF's group codes, and checks two things apart:

- the look: each architectural object (wall, opening, space, grid, column, stair, ramp, tag, view mark) is one INSERT of
  model space at the origin, unturned and unscaled, on the object's layer, of an anonymous block (`*U…`, flag 1) that
  draws something (lines, polylines, hatches, texts, circles, arcs, solids); the block's drawn points are where the
  object is — a space's block holds its rings exactly, a wall's lies within its width of its base line, the others hold
  the object's own points (a grid's ends, a column's centre, a stair's and a ramp's walking line, a tag's middle, a view
  mark's points, an opening's frame);
- the meaning: the INSERT's KENTOS data carries the object as the contract's JSON, the same object the drawing holds
  (its slot number aside), and its persistent id; an opening's wall, a column's grids and a tag's object are named by the
  persistent ids of the other objects the file carries.

`--check` is the only mode; the files are KentOS's output, this reads them.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402  (the independent reader)

DIR = ROOT / "fixtures/formats/v1/dxf-arch"
ARCH = {"wall", "opening", "space", "grid", "column", "stair", "ramp", "tag", "viewmark"}
DRAWN = {"LINE", "LWPOLYLINE", "POLYLINE", "HATCH", "TEXT", "MTEXT", "CIRCLE", "ARC", "SOLID", "ELLIPSE", "INSERT"}


def pairs(path):
    lines = path.read_text(encoding="utf-8").splitlines()
    return [(int(lines[i].strip()), lines[i + 1]) for i in range(0, len(lines) - 1, 2)]


def records(ps):
    """The file's records in order: (type, [(code, value)…]), the section names as records of their own."""
    out = []
    for code, value in ps:
        if code == 0:
            out.append((value, []))
        elif out:
            out[-1][1].append((code, value))
    return out


def caret(s):
    out, i = [], 0
    while i < len(s):
        if s[i] == "^" and i + 1 < len(s):
            out.append("^" if s[i + 1] == " " else chr(ord(s[i + 1]) - 64))
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def kentos_items(groups):
    """The KENTOS application's items: tag → its values (a long string's pieces joined)."""
    items, k = {}, 0
    xs = [(c, v) for c, v in groups if c >= 1000]
    start = next((i for i, (c, v) in enumerate(xs) if c == 1001 and v == "KENTOS"), None)
    if start is None:
        return items
    xs = xs[start + 1:]
    while k < len(xs):
        if xs[k] == (1002, "{") and k + 1 < len(xs) and xs[k + 1][0] == 1000:
            tag, k = xs[k + 1][1], k + 2
            values, depth, piece = [], 0, []
            while k < len(xs) and not (xs[k] == (1002, "}") and depth == 0):
                c, v = xs[k]
                if (c, v) == (1002, "{"):
                    depth, piece = depth + 1, []
                elif (c, v) == (1002, "}"):
                    depth -= 1
                    values.append("".join(piece))
                elif depth:
                    piece.append(caret(v))
                else:
                    values.append(caret(v) if c == 1000 else v)
                k += 1
            items[tag] = values
        k += 1
    return items


def points_of(groups):
    xs = [float(v) for c, v in groups if c in (10, 11, 12, 13)]
    ys = [float(v) for c, v in groups if c in (20, 21, 22, 23)]
    return list(zip(xs, ys))


def box(pts):
    return [min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)]


def inside(p, b, slack):
    return b[0] - slack <= p[0] <= b[2] + slack and b[1] - slack <= p[1] <= b[3] + slack


def own_points(e):
    """The object's own points, and how far its drawing may lie from them."""
    k = e["kind"]
    xy = lambda p: (p["x"], p["y"])  # noqa: E731
    if k == "wall":
        return [xy(p) for p in e["pts"]], e.get("width", 0.25)
    if k == "space":
        return [xy(p) for part in e["parts"] for p in part["outer"]["pts"]], 0.0
    if k in ("grid", "stair", "ramp"):
        return [xy(p) for p in e["pts"]], 0.0
    if k == "column":
        return [xy(e["at"])], 0.0
    if k == "tag":
        return [xy(e["at"])], 0.0
    if k == "viewmark":
        return [xy(p) for p in e["points"]], 0.0
    if k == "opening":
        return [xy(p) for p in e.get("place", [])], 0.0
    raise AssertionError(k)


def check():
    _, doc = kcad.read((DIR / "mimari.kcad").read_bytes())
    uids = dict(zip((e["id"] for e in doc["entities"]), doc["uids"]))
    names = {}

    def walk(nodes):
        for n in nodes:
            names[n["id"]] = n["name"]
            walk(n["children"])

    walk(doc["layers"])
    arch = [e for e in doc["entities"] if e["kind"] in ARCH]
    assert {e["kind"] for e in arch} == ARCH, {e["kind"] for e in arch}
    recs = records(pairs(DIR / "mimari.dxf"))
    blocks, section, current = {}, None, None
    inserts = []
    for kind, groups in recs:
        if kind == "SECTION":
            section = dict(groups).get(2)
            continue
        if section == "BLOCKS":
            if kind == "BLOCK":
                g = dict(groups)
                current = g.get(2)
                blocks[current] = {"flags": int(g.get(70, "0")), "drawn": []}
            elif kind == "ENDBLK":
                current = None
            elif current is not None:
                blocks[current]["drawn"].append((kind, groups))
        elif section == "ENTITIES" and kind == "INSERT":
            items = kentos_items(groups)
            if "object" in items:
                inserts.append((groups, json.loads(items["object"][0])))
    problems = []
    by_uid = {}
    for groups, obj in inserts:
        g = dict(groups)
        uid = obj.pop("uid", None)
        by_uid[uid] = (g, obj)
    assert len(by_uid) == len(inserts), "one INSERT per object"
    if {o["kind"] for _, o in by_uid.values()} - ARCH:
        problems.append("KENTOS verisinde mimari olmayan nesne")
    for e in arch:
        uid = uids[e["id"]]
        if uid not in by_uid:
            problems.append(f"{e['kind']} {uid}: INSERT'i yok")
            continue
        g, obj = by_uid[uid]
        # The meaning: the same object, its slot number aside.
        want = {k: v for k, v in e.items() if k != "id"}
        got = {k: v for k, v in obj.items() if k != "id"}
        if want != got:
            diff = sorted(k for k in set(want) | set(got) if want.get(k) != got.get(k))
            problems.append(f"{e['kind']} {uid}: KENTOS verisindeki nesne farklı: {diff}")
        for key in ("host", "of"):
            if key in e and e[key] not in uids.values():
                problems.append(f"{e['kind']} {uid}: {key} çizimdeki bir nesne değil")
        for gr in e.get("grids") or []:
            if gr not in uids.values():
                problems.append(f"{e['kind']} {uid}: aks çizimde değil")
        # The look: an INSERT at the origin of an anonymous block that draws the object.
        if (float(g.get(10, 0)), float(g.get(20, 0))) != (0.0, 0.0):
            problems.append(f"{e['kind']} {uid}: INSERT başlangıçta değil")
        for code in (41, 42, 50):
            if code in g and float(g[code]) != (0.0 if code == 50 else 1.0):
                problems.append(f"{e['kind']} {uid}: INSERT dönük ya da ölçekli ({code})")
        if g.get(8) != names.get(e["layerId"]):
            problems.append(f"{e['kind']} {uid}: katmanı {g.get(8)} (beklenen {names.get(e['layerId'])})")
        name = g.get(2, "")
        b = blocks.get(name)
        if not name.startswith("*U") or b is None or not b["flags"] & 1:
            problems.append(f"{e['kind']} {uid}: adsız blok değil ({name})")
            continue
        drawn = [(k, gs) for k, gs in b["drawn"] if k in DRAWN]
        if not drawn:
            problems.append(f"{e['kind']} {uid}: bloğu bir şey çizmiyor")
            continue
        pts = [p for _, gs in drawn for p in points_of(gs)]
        own, slack = own_points(e)
        bb = box(pts)
        for p in own:
            if not inside(p, bb, slack + 1e-9):
                problems.append(f"{e['kind']} {uid}: {p} bloğun çizdiğinin dışında {bb}")
        if e["kind"] == "space":
            for part in e["parts"]:
                ring = [(p["x"], p["y"]) for p in part["outer"]["pts"]]
                polys = [points_of(gs) for k, gs in drawn if k == "LWPOLYLINE"]
                if not any(sorted(r) == sorted(ring) for r in polys):
                    problems.append(f"space {uid}: halkası bloğunda yok")
        elif e["kind"] == "wall":
            bw = box(own)
            if not all(inside(p, bw, slack) for p in pts):
                problems.append(f"wall {uid}: bloğu duvarın genişliğinden taşıyor")
    for p in problems:
        print(p)
    print(f"{len(arch)} mimari nesne, {len(inserts)} INSERT: {'sorun yok' if not problems else f'{len(problems)} sorun'}.")
    return 1 if problems else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.parse_args()
    return check()


if __name__ == "__main__":
    sys.exit(main())
