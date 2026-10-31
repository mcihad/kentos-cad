#!/usr/bin/env python3
"""Independent cases for cad.walls.rectangle's `adopt` (docs/adr/0770 §1: Ortak duvar).

Only the ADR and the command's public contract are used; no KentOS module or output. The room's ring is
walls_rectangle_cases.py's independent 80-digit frame (counter-clockwise, its left the room). An existing wall's lines
are its path offset to the left (mitred at its corners, square at an open path's ends): the left face at the offset
plus 0 / half the width / the width (left / centre / right justification), the right face a width less, the axis
halfway. A side of the room is covered where an existing straight edge on the same level runs parallel to it (within
0.001°) and the edge's line of the room's kind lies on the side within 1 mm: Inner the face towards the room (the one
further along the side's left normal), Centre the axis, Outer the face away from it. The covered stretch is the wall's
body there (its two faces' edges) projected on the side, clipped to it, kept when longer than 1 mm. Stretches round the whole path are merged (a gap
of 1 mm or less closes; the last joins the first across the path's start); what lies between them, longer than 1 mm,
is a new open wall: the point where a covered stretch ends, the ring's corners strictly between, the point where the
next begins, in the order of the stretches. Nothing covered: the one closed wall; everything covered: refused.

    python3 scripts/fixtures/wall_adopt_cases.py
    python3 scripts/fixtures/wall_adopt_cases.py --check
"""

import argparse
import copy
import json
import sys
from decimal import Decimal, getcontext
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import walls_rectangle_cases as wr  # noqa: E402

getcontext().prec = 80
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.walls.rectangle.adopt.json"
COMMAND = "cad.walls.rectangle"
STEP = "Duvar"
TOL = Decimal("1e-3")
PARALLEL = Decimal("1.745329251994330e-5")
ALL_COVERED = "Dikdörtgenin bütün kenarları var olan duvarlarda; çizilecek yeni duvar kalmadı."

SETUP = copy.deepcopy(wr.SETUP)
SETUP["settings"]["arch"] = {"levels": [{"id": "zemin", "name": "Zemin", "elevation": 0.0, "height": 3.0},
                                        {"id": "kat1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}
SENTINEL = wr.SENTINEL


def D(v):
    return Decimal(v) if isinstance(v, int) else Decimal.from_float(float(v))


def P(p):
    return (D(p["x"]), D(p["y"]))


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def length(a):
    return (a[0] * a[0] + a[1] * a[1]).sqrt()


def unit(a, b):
    d = sub(b, a)
    n = length(d)
    return (d[0] / n, d[1] / n)


def left(u):
    return (-u[1], u[0])


def offset_line(pts, closed, o):
    """The path offset `o` to its left: each edge moved square to itself, the corners where the moved edges meet."""
    n = len(pts)
    edges = n if closed else n - 1
    moved = []
    for i in range(edges):
        a, b = pts[i], pts[(i + 1) % n]
        nn = left(unit(a, b))
        moved.append(((a[0] + nn[0] * o, a[1] + nn[1] * o), (b[0] + nn[0] * o, b[1] + nn[1] * o)))
    out = []
    for i in range(n):
        before = moved[i - 1] if (closed or i > 0) else None
        after = moved[i] if (closed or i < n - 1) else None
        if before is None:
            out.append(after[0])
        elif after is None:
            out.append(before[1])
        else:
            (p, q), (r, s) = before, after
            d1, d2 = sub(q, p), sub(s, r)
            den = cross(d1, d2)
            if den == 0:
                out.append(after[0])
            else:
                t = cross(sub(r, p), d2) / den
                out.append((p[0] + d1[0] * t, p[1] + d1[1] * t))
    return out


def faces(w):
    width = D(w["width"])
    off = D(w.get("offset", 0))
    lf = off + {"left": Decimal(0), "center": width / 2, "right": width}[w["justify"]]
    return lf, lf - width


def covered(a, b, reference, walls):
    """The covered stretches of side a → b (along it from a) and the walls (by their index) covering it."""
    total = length(sub(b, a))
    u = unit(a, b)
    nn = left(u)
    out, by = [], []
    for j, w in enumerate(walls):
        pts = [P(p) for p in w["pts"]]
        closed = w.get("closed", False)
        bulges = w.get("bulges")
        lf, rf = faces(w)
        offsets = [lf, rf, (lf + rf) / 2]
        lines = [offset_line(pts, closed, o) for o in offsets]
        edges = len(pts) if closed else len(pts) - 1
        for e in range(edges):
            if bulges and bulges[e] != 0:
                continue
            c, d = pts[e], pts[(e + 1) % len(pts)]
            v = unit(c, d)
            if abs(cross(u, v)) > PARALLEL:
                continue
            ne = left(v)
            s = lambda o: dot(nn, sub((c[0] + ne[0] * o, c[1] + ne[1] * o), a))  # noqa: E731
            sl, sr = s(lf), s(rf)
            if reference == "inner":
                k, at = (0, sl) if sl >= sr else (1, sr)
            elif reference == "outer":
                k, at = (0, sl) if sl <= sr else (1, sr)
            else:
                k, at = 2, s((lf + rf) / 2)
            if abs(at) > TOL:
                continue
            ts = [dot(u, sub(p, a)) for p in (lines[0][e], lines[0][(e + 1) % len(pts)], lines[1][e], lines[1][(e + 1) % len(pts)])]
            lo, hi = max(min(ts), Decimal(0)), min(max(ts), total)
            if hi - lo > TOL:
                out.append((lo, hi))
                if j not in by:
                    by.append(j)
    return out, by


def adopt(ring, reference, walls):
    """What is left of the ring (paths, closed) and the walls adopted, by the module docstring's rule."""
    m = len(ring)
    start = [Decimal(0)]
    cover, adopted = [], []
    for k in range(m):
        a, b = ring[k], ring[(k + 1) % m]
        start.append(start[k] + length(sub(b, a)))
        c, by = covered(a, b, reference, walls)
        cover += [(start[k] + x, start[k] + y) for x, y in c]
        adopted += [j for j in by if j not in adopted]
    total = start[m]
    if not cover:
        return [ring], True, []
    cover.sort(key=lambda x: x[0])
    merged = []
    for x, y in cover:
        if merged and x <= merged[-1][1] + TOL:
            merged[-1] = (merged[-1][0], max(merged[-1][1], y))
        else:
            merged.append((x, y))
    if len(merged) > 1 and merged[0][0] <= TOL and merged[-1][1] >= total - TOL:
        first = merged.pop(0)
        merged[-1] = (merged[-1][0], total + first[1])

    def point(t):
        t = t % total
        k = max(i for i in range(m) if start[i] <= t)
        a, b = ring[k], ring[(k + 1) % m]
        span = start[k + 1] - start[k]
        f = (t - start[k]) / span if span > 0 else Decimal(0)
        return (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f)

    paths = []
    for i, (_, end) in enumerate(merged):
        nxt = merged[i + 1][0] if i + 1 < len(merged) else merged[0][0] + total
        if nxt - end <= TOL:
            continue
        path = [point(end)]
        for k in range(2 * m):
            corner = start[k % m] + total * (k // m)
            if end + TOL < corner < nxt - TOL:
                path.append(ring[k % m])
        path.append(point(nxt))
        paths.append(path)
    return paths, False, sorted(adopted)


def F(p):
    return wr.point(float(p[0]), float(p[1]))


def wall(id_, pts, width=0.2, justify="left", closed=False, **more):
    e = {"kind": "wall", "id": id_, "layerId": more.pop("layerId", "duvar"), "attrs": {}, "pts": pts}
    if closed:
        e["closed"] = True
    e["width"] = width
    e["justify"] = justify
    e.update(more)
    return e


def room(id_, a, b, reference="inner", width=0.2, rotation=0, **more):
    """An existing room drawn by cad.walls.rectangle: its ring by the independent frame."""
    frame = {"kind": "corners", "a": wr.point(*a), "b": wr.point(*b)}
    pts, _, _ = wr.path(frame, rotation)
    return wall(id_, pts, width, {"inner": "left", "outer": "right", "center": "center"}[reference], True, **more)


def rect(a, b, **kwargs):
    inp = {"layerId": "duvar", "frame": {"kind": "corners", "a": wr.point(*a), "b": wr.point(*b)}, "adopt": True}
    inp.update(kwargs)
    return inp


def expected(inp, existing, first_id):
    """The new walls as written and the adopted walls' ids; none when everything is covered."""
    pts, _, _ = wr.path(inp["frame"], inp.get("rotation", 0))
    ring = [P(p) for p in pts]
    reference = inp.get("reference", "inner")
    level = (inp.get("arch") or {}).get("level")
    same = [w for w in existing if (w.get("arch") or {}).get("level") == level]
    if not inp.get("adopt"):
        paths, closed, by = [ring], True, []
    else:
        paths, closed, by = adopt(ring, reference, same)
    if not paths:
        return None
    style = wr.style_of(inp)
    thickness = inp.get("width", (0.2 if style is wr.FIXED else style["width"]) if style else 0.2)
    out = []
    for k, path in enumerate(paths):
        e = wall(first_id + k, [F(p) for p in path], thickness,
                 {"inner": "left", "outer": "right", "center": "center"}[reference], closed)
        e["layerId"] = inp["layerId"]
        for field in ["arch"]:
            if field in inp:
                e[field] = copy.deepcopy(inp[field])
        if style:
            e["style"] = style["id"]
            e["components"] = copy.deepcopy(style["components"])
        out.append(e)
    return out, [same[j]["id"] for j in by]


def case(name, existing, inp, op="execute", undo=False, note=None):
    setup = copy.deepcopy(SETUP)
    setup["entities"] = [SENTINEL] + existing
    before_ids = [1] + [w["id"] for w in existing]
    before_entities = {str(e["id"]): e for e in setup["entities"]}
    first = len(before_ids) + 1
    made = expected(inp, existing, first)
    if made is None:
        step = {"op": op, "input": inp, "result": {"status": "failed", "error": {
            "code": "nothing_to_set", "message": ALL_COVERED, "path": "frame"}},
            "expect": {"ids": before_ids, "entities": before_entities, "canUndo": False, "revision": "same"}}
        out = {"name": name, "setup": setup, "steps": [step]}
        if note:
            out["note"] = note
        return out
    walls, adopted = made
    ids = [w["id"] for w in walls]
    if op == "plan":
        output = {"walls": [{**w, "id": 0} for w in walls], "revision": "$current"}
        if adopted:
            output["adopted"] = [f"$uidOf:{j}" for j in adopted]
        step = {"op": "plan", "input": inp, "within": 1e-9, "result": {"status": "completed", "output": output, "warnings": []},
                "expect": {"ids": before_ids, "entities": before_entities, "canUndo": False, "revision": "same"}}
        steps = [step]
    else:
        output = {"uids": ["$any"] * len(walls), "ids": ids, "revision": "$current"}
        if adopted:
            output["adopted"] = [f"$uidOf:{j}" for j in adopted]
        after = dict(before_entities)
        after.update({str(w["id"]): w for w in walls})
        steps = [{"op": "execute", "input": inp, "within": 1e-9,
                  "result": {"status": "completed", "output": output, "warnings": "$any"},
                  "expect": {"ids": before_ids + ids, "entities": after, "canUndo": True, "canRedo": False,
                             "revision": "changed"}}]
        if undo:
            steps.append({"op": "undo", "returns": STEP, "expect": {"ids": before_ids, "entities": before_entities,
                                                                     "canRedo": True, "revision": "changed"}})
    out = {"name": name, "setup": setup, "steps": steps}
    if note:
        out["note"] = note
    return out


def cases():
    A = room(2, (0, 0), (4, 3))
    B = room(3, (7.4, 0), (10.4, 3))
    out = [
        case("Sağdaki 3×3 oda iç net: ortak duvar benimsenir, üç kenar tek açık duvar", [A], rect((4.2, 0), (7.2, 3))),
        case("Üstteki oda: üst duvarın dış yüzü benimsenir", [A], rect((0, 3.2), (4, 6.2))),
        case("Ortak duvardan yüksek oda: kalan sol parça dış köşeye kadar çizilir", [A], rect((4.2, 0), (7.2, 4))),
        case("İki oda arasında: iki yan benimsenir, alt ve üst iki ayrı duvar", [A, B], rect((4.2, 0), (7.2, 3))),
        case("L köşe: alt ve sağ benimsenir, kalan L tek duvar",
             [A, room(3, (4.2, 0), (6, 6.4))], rect((0, 3.2), (4, 6.2))),
        case("Komşusu yok: tek kapalı duvar, benimsenen yok", [A], rect((10, 10), (13, 13))),
        case("5 mm boşluk: tolerans dışında, kapalı duvar", [A], rect((4.205, 0), (7.205, 3))),
        case("0,5 mm içinde: benimsenir, yeni duvar kendi halkasında", [A], rect((4.2005, 0), (7.2005, 3))),
        case("Başka kattaki duvar benimsenmez", [room(2, (0, 0), (4, 3), arch={"level": "zemin"})],
             rect((4.2, 0), (7.2, 3), arch={"level": "kat1"})),
        case("Aynı kattaki duvar benimsenir", [room(2, (0, 0), (4, 3), arch={"level": "zemin"})],
             rect((4.2, 0), (7.2, 3), arch={"level": "zemin"})),
        case("Kalın (0,30 m) var olan duvar: dış yüzü eşleşir", [room(2, (0, 0), (4, 3), width=0.3)], rect((4.3, 0), (7.3, 3))),
        case("Eksen referansı: ortak eksen benimsenir, uçlar köşelerde",
             [room(2, (0, 0), (4, 3), reference="center")], rect((4, 0), (7, 3), reference="center")),
        case("Dış ölçü: var olan duvarın uzak yüzü yeni odanın dış çizgisinde",
             [room(2, (0, 0), (4, 3), reference="outer")], rect((3.8, 0), (6.8, 3), reference="outer")),
        case("Dış ölçü, ortak duvar dışında: benimsenmez",
             [room(2, (0, 0), (4, 3), reference="outer")], rect((4, 0), (7, 3), reference="outer")),
        case("Aynı oda ikinci kez: bütün kenarlar dolu, ret", [A], rect((0, 0), (4, 3))),
        case("Kısa duvar alt kenarın ortasında: halka onun çevresinden tek açık duvar",
             [wall(2, [wr.point(5, 0), wr.point(6, 0)])], rect((4.2, 0), (7.2, 3))),
        case("Yay kenarlı duvar benimsenmez",
             [wall(2, [wr.point(4.2, -0.5), wr.point(4.2, 3.5)], justify="right", bulges=[0.2, 0.0])], rect((4.2, 0), (7.2, 3))),
        case("Düz duvar, yay kenarı yok: aynı yer benimsenir",
             [wall(2, [wr.point(4.2, -0.5), wr.point(4.2, 3.5)], justify="right")], rect((4.2, 0), (7.2, 3))),
        case("Paralel olmayan (dönük) oda benimsenmez",
             [room(2, (0, 0), (4, 3), rotation=0.17453292519943295)], rect((4.2, 0), (7.2, 3))),
        case("Ortak duvar istenmedi: eski davranış, kapalı duvar", [A], {**rect((4.2, 0), (7.2, 3)), "adopt": False}),
        case("Kilitli katmandaki duvar da benimsenir (değişmez)", [room(2, (0, 0), (4, 3), layerId="kilitli")],
             rect((4.2, 0), (7.2, 3))),
        case("Stilli oda: bölme stili ve bileşenleriyle açık duvar", [A], rect((4.2, 0), (7.2, 3), style="bolme-15")),
        case("Plan: yazılmaz, benimsenen ve kalan duvar söylenir", [A], rect((4.2, 0), (7.2, 3)), op="plan"),
        case("Geri al: yeni duvarlar tek adımda gider, var olan oda kalır", [A, B], rect((4.2, 0), (7.2, 3)), undo=True),
        case("Ters köşe sırası: halka saat yönü tersine düzenlenir, aynı U", [A], rect((7.2, 3), (4.2, 0))),
    ]
    # A rotated pair: the room 4.2 m along the first's rotated side.
    r = 0.5235987755982988
    s, c = wr.trig(r)
    a2 = (float(D(4.2) * c), float(D(4.2) * s))
    b2 = (float(D(a2[0]) + 3 * c - 3 * s), float(D(a2[1]) + 3 * s + 3 * c))
    rot = room(2, (0, 0), (float(4 * c - 3 * s), float(4 * s + 3 * c)), rotation=r)
    out.append(case("30° dönük iki oda: ortak duvar dönük çerçevede benimsenir", [rot], rect(a2, b2, rotation=r)))
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Dikdörtgen duvar: ortak duvarı benimseyen bitişik oda (adopt)",
            "note": "Beklenenler scripts/fixtures/wall_adopt_cases.py'nin bağımsız hesabından (ADR 0770 §1; 80 basamaklı Decimal, "
                    "halka walls_rectangle_cases.py'nin çerçevesi); KentOS kodu kullanılmadı. Tel birimi metre. 1 gözcü çizgi, "
                    "var olan duvarlar 2'den.",
            "setup": SETUP, "cases": cases()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = document()
    encoded = json.dumps(doc, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != encoded:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"{COMMAND} (adopt): {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
    else:
        OUT.write_text(encoded)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
