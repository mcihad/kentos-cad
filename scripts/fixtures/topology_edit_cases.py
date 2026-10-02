#!/usr/bin/env python3
"""Independent reference of topological editing (docs/adr/0160).

Writes fixtures/topology/v1/edit.json from the ADR's rules alone, with exact
fractions and no KentOS code. Both platforms put every case's neighbours
right with the shared core (`ops::topology_edit`, WASM `topologyEdit`) and
must give the same shapes, the same locked and the same invalid counts.

The rules (docs/adr/0160 §2–§3):

1. A neighbour's paths, in the elevations' order: a line's two ends; a
   polyline; an area's outer ring, its holes, then each further part's ring
   and holes. A point's place is a vertex only when `points` is on.
2. Two places are one within 1 µm (squared distance at most 1e-12).
3. The changes: the moves together (every vertex on a move's `at`, as it
   was before any move, goes to its `to`); then each other change in turn:
   - insert: on every straight edge (|bulge| at most 1e-12) whose ends are
     `a` and `b` (either way round), `p` between them, the two halves
     straight; a line becomes a three-vertex polyline;
   - bulge: an edge from `a` to `b` whose bulge is `from` (within 1e-9)
     takes `to`; one from `b` to `a` whose bulge is −`from` takes −`to`; a
     line becomes a two-vertex polyline when its new bulge is an arc's;
   - remove: a vertex on `at` whose two path neighbours are `prev` and
     `next` (either way round) goes; the merged edge is straight. An open
     path keeps at least two vertices, a ring three: otherwise the
     neighbour is invalid and stays as it was.
4. A neighbour that would change but lies on a locked layer stays as it was
   and is counted; so is an invalid one.

5. The changes an edit made, from the edited object before and after it
   (path by path, in the elevations' order): with as many vertices, every
   vertex that changed place is a move and every edge whose ends stayed
   and whose bulge changed is a bulge; with one vertex more, the first that
   differs is an insert on the edge it splits (not at an open path's end);
   with one fewer, the first that differs is a remove with its two path
   neighbours (not an open path's end). The moves come first.

What is compared: every changed neighbour's index and shape, bit for bit
(nothing is computed: the vertices are copies of the changes' points), the
two counts, and the changes found from an edit's before and after.
"""

import argparse
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "topology" / "v1" / "edit.json"
SAME2 = F(1, 10**12)
BULGE = F(1, 10**9)
# A bulge this small is a straight edge (geom::bulge).
STRAIGHT = F(1, 10**12)


def P(x, y):
    return {"x": x, "y": y}


def same(p, q):
    dx = F(p["x"]) - F(q["x"])
    dy = F(p["y"]) - F(q["y"])
    return dx * dx + dy * dy <= SAME2


def margin(p, q, what):
    """Two places are one or clearly apart: never at the 1 µm line."""
    dx = F(p["x"]) - F(q["x"])
    dy = F(p["y"]) - F(q["y"])
    d2 = dx * dx + dy * dy
    assert d2 <= SAME2 * F(36, 100) or d2 >= SAME2 * F(225, 100), f"{what}: {p} and {q} lie near the 1 µm line"


# ── Paths ──


def paths_of(shape):
    """The shape's paths [pts, bulges, closed] in the elevations' order, and how to put them back."""
    k = shape["kind"]
    if k == "line":
        return [[[shape["a"], shape["b"]], [0.0], False]], ("line",)
    if k == "polyline":
        pts = shape["pts"]
        return [[list(pts), padded(shape.get("bulges"), len(pts) - 1), False]], ("polyline",)
    if k == "polygon":
        out, plan = [], []
        rings = [shape] + list(shape.get("parts") or [])
        for r in rings:
            out.append([list(r["pts"]), padded(r.get("bulges"), len(r["pts"])), True])
            holes = r.get("holes") or []
            for h in holes:
                out.append([list(h["pts"]), padded(h.get("bulges"), len(h["pts"])), True])
            plan.append(len(holes))
        return out, ("polygon", plan)
    return None, None


def padded(bulges, n):
    b = list(bulges or [])
    return (b + [0.0] * n)[:n]


def shape_of(paths, plan, before):
    kind = plan[0]
    if kind in ("line", "polyline"):
        pts, bulges, _ = paths[0]
        if kind == "line" and len(pts) == 2 and all(b == 0 for b in bulges):
            return {"kind": "line", "a": pts[0], "b": pts[1]}
        out = {"kind": "polyline", "pts": pts, "bulges": bulges}
        if before.get("holes"):
            out["holes"] = before["holes"]
        return out
    holes_per = plan[1]
    at = 0
    rings = []
    for count in holes_per:
        ring = paths[at]
        holes = paths[at + 1 : at + 1 + count]
        at += 1 + count
        r = {"pts": ring[0], "bulges": ring[1]}
        if count:
            r["holes"] = [{"pts": h[0], "bulges": h[1]} for h in holes]
        rings.append(r)
    out = {"kind": "polygon", **rings[0]}
    if len(rings) > 1:
        out["parts"] = rings[1:]
    return out


# ── Changes ──


def apply(case):
    moves = [c for c in case["changes"] if c["kind"] == "move"]
    others = [c for c in case["changes"] if c["kind"] != "move"]
    edited, locked, invalid = [], 0, 0
    for i, n in enumerate(case["neighbours"]):
        shape = n["shape"]
        if shape["kind"] == "point":
            if not case["points"]:
                continue
            hit = next((m for m in moves if same(shape["p"], m["at"])), None)
            if hit is None:
                continue
            if n.get("locked"):
                locked += 1
                continue
            edited.append({"index": i, "shape": {**shape, "p": hit["to"]}})
            continue
        paths, plan = paths_of(shape)
        if paths is None:
            continue
        changed, bad = False, False
        # The moves together, against the places as they were.
        for path in paths:
            for k, v in enumerate(path[0]):
                for m in moves:
                    margin(v, m["at"], "move")
                hit = next((m for m in moves if same(v, m["at"])), None)
                if hit is not None:
                    path[0][k] = hit["to"]
                    changed = True
        for c in others:
            for path in paths:
                pts, bulges, closed = path
                count = len(pts) if closed else len(pts) - 1
                if c["kind"] == "insert":
                    for e in reversed(range(count)):
                        u, w = pts[e], pts[(e + 1) % len(pts)]
                        for q in (c["a"], c["b"]):
                            margin(u, q, "insert")
                            margin(w, q, "insert")
                        on = (same(u, c["a"]) and same(w, c["b"])) or (same(u, c["b"]) and same(w, c["a"]))
                        if on and abs(F(bulges[e])) <= STRAIGHT:
                            pts.insert(e + 1, c["p"])
                            bulges[e : e + 1] = [0.0, 0.0]
                            changed = True
                elif c["kind"] == "bulge":
                    for e in range(count):
                        u, w = pts[e], pts[(e + 1) % len(pts)]
                        if same(u, c["a"]) and same(w, c["b"]) and abs(F(bulges[e]) - F(c["from"])) <= BULGE:
                            bulges[e] = c["to"]
                            changed = True
                        elif same(u, c["b"]) and same(w, c["a"]) and abs(F(bulges[e]) + F(c["from"])) <= BULGE:
                            bulges[e] = -c["to"] if c["to"] != 0 else 0.0
                            changed = True
                elif c["kind"] == "remove":
                    n_pts = len(pts)
                    for k in range(n_pts):
                        if not same(pts[k], c["at"]):
                            continue
                        if closed:
                            before, after = pts[(k - 1) % n_pts], pts[(k + 1) % n_pts]
                        else:
                            if k == 0 or k == n_pts - 1:
                                continue
                            before, after = pts[k - 1], pts[k + 1]
                        if not ((same(before, c["prev"]) and same(after, c["next"])) or (same(before, c["next"]) and same(after, c["prev"]))):
                            continue
                        if n_pts - 1 < (3 if closed else 2):
                            bad = True
                            break
                        prev_edge = (k - 1) % n_pts if closed else k - 1
                        bulges[prev_edge] = 0.0
                        del pts[k]
                        del bulges[k if k < len(bulges) else -1]
                        changed = True
                        break
        if not changed and not bad:
            continue
        if n.get("locked"):
            locked += 1
            continue
        if bad:
            invalid += 1
            continue
        edited.append({"index": i, "shape": shape_of(paths, plan, shape)})
    return {"edited": edited, "locked": locked, "invalid": invalid}


# ── The changes an edit made ──


def diff(before, after):
    pb, _ = paths_of(before)
    pa, _ = paths_of(after)
    if pb is None or pa is None or len(pb) != len(pa):
        return []
    moves, others = [], []
    for (bp, bb, closed), (ap, ab, _) in zip(pb, pa):
        n, m = len(bp), len(ap)
        if m == n:
            for k in range(n):
                if bp[k] != ap[k]:
                    moves.append({"kind": "move", "at": bp[k], "to": ap[k]})
            for e in range(n if closed else n - 1):
                u, w = bp[e], bp[(e + 1) % n]
                if u == ap[e] and w == ap[(e + 1) % n] and bb[e] != ab[e]:
                    others.append({"kind": "bulge", "a": u, "b": w, "from": bb[e], "to": ab[e]})
        elif m == n + 1:
            j = next((k for k in range(n) if bp[k] != ap[k]), n)
            if closed:
                others.append({"kind": "insert", "a": bp[(j - 1) % n], "b": bp[j % n], "p": ap[j]})
            elif 0 < j < n:
                others.append({"kind": "insert", "a": bp[j - 1], "b": bp[j], "p": ap[j]})
        elif m == n - 1:
            j = next((k for k in range(m) if bp[k] != ap[k]), m)
            if closed:
                others.append({"kind": "remove", "at": bp[j], "prev": bp[(j - 1) % n], "next": bp[(j + 1) % n]})
            elif 0 < j < n - 1:
                others.append({"kind": "remove", "at": bp[j], "prev": bp[j - 1], "next": bp[j + 1]})
    return moves + others


def diffs():
    sq = square(0.0, 0.0, 10.0, 10.0)
    return [
        ("Tutamaçla köşe taşıma", sq, {**sq, "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.5, 9.75), P(0.0, 10.0)]}),
        ("Kenar ortasından yeni köşe (kapanış kenarı)", sq, {**sq, "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 10.0), P(0.0, 10.0), P(-0.5, 5.0)], "bulges": [0.0] * 5}),
        ("Çizgiye köşe: çoklu çizgi olur", {"kind": "line", "a": P(0.0, 0.0), "b": P(10.0, 0.0)}, {"kind": "polyline", "pts": [P(0.0, 0.0), P(5.0, 0.5), P(10.0, 0.0)], "bulges": [0.0, 0.0]}),
        ("Yaylı kenarın ortası: kabarıklık değişir", {**sq, "bulges": [0.0, 0.3, 0.0, 0.0]}, {**sq, "bulges": [0.0, 0.5, 0.0, 0.0]}),
        ("Köşe sil", {"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 5.0), P(10.0, 10.0), P(0.0, 10.0)]}, {"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 10.0), P(0.0, 10.0)], "bulges": [0.0] * 4}),
        ("Esnet: iki köşe birlikte", sq, {**sq, "pts": [P(0.0, 0.0), P(12.0, 0.0), P(12.0, 10.0), P(0.0, 10.0)]}),
        ("Açık yolun ucuna köşe: kenar ekleme değil", {"kind": "polyline", "pts": [P(0.0, 0.0), P(10.0, 0.0)]}, {"kind": "polyline", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(15.0, 0.0)]}),
    ]


# ── Cases ──


def square(x0, y0, x1, y1):
    return {"kind": "polygon", "pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}


def n(shape, locked=False):
    out = {"shape": shape}
    if locked:
        out["locked"] = True
    return out


def cases():
    out = []
    corner = P(10.0, 10.0)
    near = P(10.0000004, 9.9999997)

    out.append(
        (
            "Üç parselin ve bir çizginin ortak köşesi taşınır; nokta kapalıyken yerinde kalır",
            [
                n(square(0.0, 0.0, 10.0, 10.0)),
                n(square(10.0, 0.0, 20.0, 10.0)),
                n({"kind": "polygon", "pts": [P(10.0000004, 9.9999997), P(20.0, 10.0), P(15.0, 20.0)]}),
                n({"kind": "line", "a": P(0.0, 20.0), "b": P(10.0, 10.0)}),
                n({"kind": "point", "p": P(10.0, 10.0)}),
                n(square(30.0, 30.0, 40.0, 40.0)),
            ],
            [{"kind": "move", "at": corner, "to": P(10.5, 9.75)}],
            False,
        )
    )
    del near
    out.append(
        (
            "Noktalar da açıkken nokta da ortak köşedir",
            [n(square(0.0, 0.0, 10.0, 10.0)), n({"kind": "point", "p": P(10.0, 10.0), "z": 101.5})],
            [{"kind": "move", "at": corner, "to": P(10.5, 9.75)}],
            True,
        )
    )
    out.append(
        (
            "Ortak kenara köşe eklenir: iki yönde de, çizgi çoklu çizgi olur, kenarı paylaşmayan dokunmaz",
            [
                n(square(0.0, 0.0, 10.0, 10.0)),
                n({"kind": "polygon", "pts": [P(10.0, 10.0), P(10.0, 0.0), P(20.0, 0.0), P(20.0, 10.0)]}),
                n({"kind": "line", "a": P(10.0, 0.0), "b": P(10.0, 10.0)}),
                n({"kind": "line", "a": P(10.0, 0.0), "b": P(10.0, 20.0)}),
            ],
            [{"kind": "insert", "a": P(10.0, 0.0), "b": P(10.0, 10.0), "p": P(10.25, 5.0)}],
            False,
        )
    )
    out.append(
        (
            "Yaylı ortak kenar aynı yaydan geçer: aynı yönde aynı, ters yönde ters işaret",
            [
                {"shape": {"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 10.0), P(0.0, 10.0)], "bulges": [0.0, 0.3, 0.0, 0.0]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 10.0), P(10.0, 0.0), P(20.0, 0.0), P(20.0, 10.0)], "bulges": [-0.3, 0.0, 0.0, 0.0]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 10.0), P(10.0, 0.0), P(20.0, 5.0)], "bulges": [0.3, 0.0, 0.0]}},
            ],
            [{"kind": "bulge", "a": P(10.0, 0.0), "b": P(10.0, 10.0), "from": 0.3, "to": 0.5}],
            False,
        )
    )
    out.append(
        (
            "Düz kenar yap: ortak yaylı kenar iki yönde de düzleşir",
            [
                {"shape": {"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 10.0), P(0.0, 10.0)], "bulges": [0.0, 0.3, 0.0, 0.0]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 10.0), P(10.0, 0.0), P(20.0, 0.0), P(20.0, 10.0)], "bulges": [-0.3, 0.0, 0.0, 0.0]}},
            ],
            [{"kind": "bulge", "a": P(10.0, 0.0), "b": P(10.0, 10.0), "from": 0.3, "to": 0.0}],
            False,
        )
    )
    out.append(
        (
            "Yaya dönüştür: ortak düz kenar yay olur, çizgi iki köşeli yaylı çoklu çizgi olur",
            [
                n(square(10.0, 0.0, 20.0, 10.0)),
                n({"kind": "line", "a": P(10.0, 10.0), "b": P(10.0, 0.0)}),
            ],
            [{"kind": "bulge", "a": P(10.0, 0.0), "b": P(10.0, 10.0), "from": 0.0, "to": 0.4}],
            False,
        )
    )
    out.append(
        (
            "Delik ve parça köşesi: adanın köşesi deliğin köşesiyle, çok parçalı alanın ikinci parçasının köşesi birlikte taşınır",
            [
                {"shape": {"kind": "polygon", "pts": [P(0.0, 0.0), P(40.0, 0.0), P(40.0, 40.0), P(0.0, 40.0)], "holes": [{"pts": [P(10.0, 10.0), P(20.0, 10.0), P(20.0, 20.0), P(10.0, 20.0)]}]}},
                n(square(10.0, 10.0, 20.0, 20.0)),
                {"shape": {"kind": "polygon", "pts": [P(50.0, 0.0), P(60.0, 0.0), P(60.0, 5.0)], "parts": [{"pts": [P(20.0, 20.0), P(30.0, 20.0), P(30.0, 30.0)]}]}},
            ],
            [{"kind": "move", "at": P(20.0, 20.0), "to": P(21.0, 19.5)}],
            False,
        )
    )
    out.append(
        (
            "Kilitli komşu değişmez, sayılır",
            [n(square(0.0, 0.0, 10.0, 10.0)), n(square(10.0, 0.0, 20.0, 10.0), locked=True)],
            [{"kind": "move", "at": P(10.0, 10.0), "to": P(10.5, 10.5)}],
            False,
        )
    )
    out.append(
        (
            "Köşe silmede sınır zinciri: iki yanı da ortak olan komşudan silinir, yalnız köşeyi paylaşandan silinmez",
            [
                {"shape": {"kind": "polygon", "pts": [P(0.0, 0.0), P(10.0, 0.0), P(10.0, 5.0), P(10.0, 10.0), P(0.0, 10.0)]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 10.0), P(10.0, 5.0), P(10.0, 0.0), P(20.0, 0.0), P(20.0, 10.0)]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 5.0), P(15.0, 2.0), P(15.0, 8.0)]}},
            ],
            [{"kind": "remove", "at": P(10.0, 5.0), "prev": P(10.0, 0.0), "next": P(10.0, 10.0)}],
            False,
        )
    )
    out.append(
        (
            "Geçersiz kalacak komşu: üçgenden köşe silinmez, sayılır",
            [
                {"shape": {"kind": "polygon", "pts": [P(10.0, 0.0), P(12.0, 5.0), P(10.0, 10.0), P(20.0, 10.0), P(20.0, 0.0)]}},
                {"shape": {"kind": "polygon", "pts": [P(10.0, 0.0), P(12.0, 5.0), P(10.0, 10.0)]}},
            ],
            [{"kind": "remove", "at": P(12.0, 5.0), "prev": P(10.0, 0.0), "next": P(10.0, 10.0)}],
            False,
        )
    )
    out.append(
        (
            "Esnet: köşeler birlikte taşınır, biri öbürünün eski yerine gelse de zincirlenmez",
            [
                n(square(0.0, 0.0, 10.0, 10.0)),
                {"shape": {"kind": "polyline", "pts": [P(10.0, 0.0), P(10.0, 10.0), P(10.0, 20.0)], "bulges": [0.0, 0.0]}},
            ],
            [
                {"kind": "move", "at": P(10.0, 0.0), "to": P(10.0, 10.0)},
                {"kind": "move", "at": P(10.0, 10.0), "to": P(10.0, 20.0)},
            ],
            False,
        )
    )
    return out


def build():
    out = []
    for name, neighbours, changes, points in cases():
        case = {"neighbours": neighbours, "changes": changes, "points": points}
        out.append({"name": name, **case, "expected": apply(json.loads(json.dumps(case)))})
    return {
        "format": "kentos.topology-edit",
        "version": 1,
        "description": "Topolojik düzenleme (docs/adr/0160): ortak köşe ve kenarların komşularda düzenlenmesi (taşıma, köşe ekleme, kabarıklık, köşe silme); değişen komşuların biçimleri, kilitli ve geçersiz sayıları; bir düzenlemenin öncesinden ve sonrasından çıkan değişiklikler. scripts/fixtures/topology_edit_cases.py kesin kesirlerle, kurallardan yazar.",
        "cases": out,
        "diffs": [{"name": name, "before": b, "after": a, "expected": diff(b, a)} for name, b, a in diffs()],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/topology_edit_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
