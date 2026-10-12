#!/usr/bin/env python3
"""Places on objects and how they follow edits (docs/adr/0249), written from the ADR without KentOS code.

The ADR's rules are written here again:
- a place's point: a point's node (part 0 its own, k its k-th extra point), a line's start, end and middle, a circle's,
  arc's or ellipse's centre, an arc's or elliptical arc's start and end, a path's vertex (part, ring, index) and the
  middle of its edge (the chord's middle, or the arc's when the edge bulges: the chord's middle moved by half the
  bulge times the chord turned clockwise);
- following an edit: the object gone breaks (object_gone); another family breaks (kind_changed; polyline and polygon
  are one family); a place the object did not have breaks (no_such_place); with the same structure (a path's place:
  its part and ring have as many vertices and close as before; a node: as many points; any other place, as a line's
  middle or an arc's start, has no structure) the place stays and its point is
  read from the new object; else a vertex (or node) is looked for by its exact coordinates among the new object's
  vertices: once rebinds, never breaks (vertex_gone), more than once breaks (ambiguous); an edge's middle needs its two
  ends found once each as consecutive vertices of one ring, else vertex_gone (ambiguous when an end is).

Writes fixtures/subref/v1/cases.json; `--check` says whether the file is what this script writes. Vertices and
chord middles of dyadic coordinates are exact; trigonometric points (arcs, ellipses) are compared within 1e-9.
"""

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/subref/v1/cases.json"


def v(x, y):
    return {"x": x, "y": y}


def pts(*xy):
    return [v(x, y) for x, y in xy]


# ---- the rules -------------------------------------------------------------------------------------------------

def family(e):
    k = e["kind"]
    return {"point": 0, "line": 1, "polyline": 2, "polygon": 2, "circle": 3, "arc": 4, "ellipse": 5}.get(k, 6)


def ring(e, part, r):
    """(points, bulges, closed) of a path's ring, or None."""
    if e["kind"] not in ("polyline", "polygon"):
        return None
    closed = e["kind"] == "polygon"
    if part == 0:
        p, b, holes = e["pts"], e.get("bulges"), e.get("holes")
    else:
        parts = e.get("parts") or []
        if part - 1 >= len(parts):
            return None
        q = parts[part - 1]
        p, b, holes = q["pts"], q.get("bulges"), q.get("holes")
    if r == 0:
        return p, b, closed
    holes = holes or []
    if r - 1 >= len(holes):
        return None
    h = holes[r - 1]
    return h["pts"], h.get("bulges"), True


def bulge(b, i):
    return b[i] if b and i < len(b) else 0.0


def edge_mid(a, b, k):
    return v((a["x"] + b["x"]) / 2 + (b["y"] - a["y"]) * k / 2, (a["y"] + b["y"]) / 2 - (b["x"] - a["x"]) * k / 2)


def point(e, pl):
    k, kind = e["kind"], pl["kind"]
    if kind == "node" and k == "point":
        if pl["part"] == 0:
            return e["p"]
        parts = e.get("parts") or []
        return parts[pl["part"] - 1]["p"] if pl["part"] - 1 < len(parts) else None
    if k == "line":
        if kind == "start":
            return e["a"]
        if kind == "end":
            return e["b"]
        if kind == "mid" and (pl["part"], pl["ring"], pl["index"]) == (0, 0, 0):
            return v((e["a"]["x"] + e["b"]["x"]) / 2, (e["a"]["y"] + e["b"]["y"]) / 2)
        return None
    if kind == "center" and k in ("circle", "arc", "ellipse"):
        return e["c"]
    if k == "arc" and kind in ("start", "end"):
        t = e["a0"] if kind == "start" else e["a1"]
        return v(e["c"]["x"] + e["r"] * math.cos(t), e["c"]["y"] + e["r"] * math.sin(t))
    if k == "ellipse" and kind in ("start", "end"):
        t = e["t0"] if kind == "start" else e["t1"]
        mj = e["major"]
        mn = (-mj["y"] * e["ratio"], mj["x"] * e["ratio"])
        return v(e["c"]["x"] + mj["x"] * math.cos(t) + mn[0] * math.sin(t),
                 e["c"]["y"] + mj["y"] * math.cos(t) + mn[1] * math.sin(t))
    if kind == "vertex":
        r = ring(e, pl["part"], pl["ring"])
        return r[0][pl["index"]] if r and pl["index"] < len(r[0]) else None
    if kind == "mid":
        r = ring(e, pl["part"], pl["ring"])
        if not r:
            return None
        p, b, closed = r
        n, i = len(p), pl["index"]
        if i >= n or (not closed and i + 1 >= n):
            return None
        return edge_mid(p[i], p[(i + 1) % n], bulge(b, i))
    return None


def vertices(e):
    out = []
    if e["kind"] == "point":
        out.append(({"kind": "node", "part": 0}, e["p"]))
        for k, q in enumerate(e.get("parts") or []):
            out.append(({"kind": "node", "part": k + 1}, q["p"]))
        return out
    if e["kind"] not in ("polyline", "polygon"):
        return out
    for part in range(1 + len(e.get("parts") or [])):
        r = 0
        while (rg := ring(e, part, r)) is not None:
            for i, p in enumerate(rg[0]):
                out.append(({"kind": "vertex", "part": part, "ring": r, "index": i}, p))
            r += 1
    return out


def same_structure(b, a, pl):
    # Only a path's rings and a multi-point's points have a structure; a line's middle has none.
    if pl["kind"] in ("vertex", "mid") and b["kind"] in ("polyline", "polygon"):
        rb, ra = ring(b, pl["part"], pl["ring"]), ring(a, pl["part"], pl["ring"])
        return bool(rb and ra and len(rb[0]) == len(ra[0]) and rb[2] == ra[2])
    if pl["kind"] == "node":
        def count(e):
            return 1 + len(e.get("parts") or []) if e["kind"] == "point" else 0
        return count(b) == count(a)
    return True


def found(e, want):
    hits = [pl for pl, p in vertices(e) if p["x"] == want["x"] and p["y"] == want["y"]]
    if len(hits) == 1:
        return hits[0], None
    return None, ("vertex_gone" if not hits else "ambiguous")


def follow(pl, before, after):
    if after is None:
        return {"broken": "object_gone"}
    if family(before) != family(after):
        return {"broken": "kind_changed"}
    was = point(before, pl)
    if was is None:
        return {"broken": "no_such_place"}
    if same_structure(before, after, pl):
        p = point(after, pl)
        return {"at": pl, "point": p} if p is not None else {"broken": "no_such_place"}
    if pl["kind"] in ("vertex", "node"):
        to, why = found(after, was)
        return {"at": to, "point": was} if to else {"broken": why}
    if pl["kind"] == "mid":
        p, _, _ = ring(before, pl["part"], pl["ring"])
        n = len(p)
        a, b = p[pl["index"]], p[(pl["index"] + 1) % n]
        (ta, wa), (tb, wb) = found(after, a), found(after, b)
        if not ta or not tb:
            return {"broken": "ambiguous" if "ambiguous" in (wa, wb) else "vertex_gone"}
        if (ta["part"], ta["ring"]) != (tb["part"], tb["ring"]):
            return {"broken": "vertex_gone"}
        np_, _, closed = ring(after, ta["part"], ta["ring"])
        m = len(np_)
        nxt = ta["index"] + 1 if ta["index"] + 1 < m else (0 if closed else None)
        if nxt != tb["index"]:
            return {"broken": "vertex_gone"}
        to = {"kind": "mid", "part": ta["part"], "ring": ta["ring"], "index": ta["index"]}
        return {"at": to, "point": point(after, to)}
    p = point(after, pl)
    return {"at": pl, "point": p} if p is not None else {"broken": "no_such_place"}


def place_at(e, snap, at):
    """The place a snap of `snap` stood on at `at`: the first candidate (the object's order) within a nanometre."""
    cands = []
    if snap == "endpoint":
        cands = [place("start"), place("end")] + [pl for pl, _ in vertices(e)]
    elif snap == "midpoint":
        cands = [place("mid", 0, 0, 0)] + [place("mid", pl["part"], pl["ring"], pl["index"])
                                          for pl, _ in vertices(e) if pl["kind"] == "vertex"]
    elif snap == "center":
        cands = [place("center")]
    elif snap == "node":
        cands = [pl for pl, _ in vertices(e)]
    for pl in cands:
        p = point(e, pl)
        if p is not None and abs(p["x"] - at["x"]) <= 1e-9 and abs(p["y"] - at["y"]) <= 1e-9:
            return pl
    return None


# ---- the cases -------------------------------------------------------------------------------------------------

def place(kind, part=None, ring_=None, index=None):
    d = {"kind": kind}
    if kind == "node":
        d["part"] = part
    if kind in ("vertex", "mid"):
        d.update({"part": part, "ring": ring_, "index": index})
    return d


LINE = {"kind": "line", "a": v(0, 0), "b": v(4, 0)}
ROOM = {"kind": "polygon", "pts": pts((0, 0), (4, 0), (4, 3), (0, 3))}
WALL = {"kind": "polyline", "pts": pts((0, 0), (2, 0), (2, 2), (5, 2))}
ARCED = {"kind": "polyline", "pts": pts((0, 0), (2, 0), (4, 0)), "bulges": [0.5, 0.0]}
HOLED = {"kind": "polygon", "pts": pts((0, 0), (10, 0), (10, 10), (0, 10)),
         "holes": [{"pts": pts((2, 2), (2, 4), (4, 4), (4, 2))}]}
PARTS = {"kind": "polygon", "pts": pts((0, 0), (2, 0), (2, 2), (0, 2)),
         "parts": [{"pts": pts((5, 0), (7, 0), (7, 2), (5, 2))}]}
CIRCLE = {"kind": "circle", "c": v(1, 1), "r": 2}
ARC = {"kind": "arc", "c": v(0, 0), "r": 2, "a0": 0.0, "a1": math.pi / 2}
ELLIPSE = {"kind": "ellipse", "c": v(1, 2), "major": v(3, 0), "ratio": 0.5, "t0": 0.25, "t1": 2.5}
POINTS = {"kind": "point", "p": v(1, 1), "parts": [{"p": v(2, 2)}, {"p": v(3, 3)}]}


def moved(e, dx, dy):
    def m(p):
        return v(p["x"] + dx, p["y"] + dy)
    out = json.loads(json.dumps(e))
    for k in ("a", "b", "c", "p"):
        if k in out:
            out[k] = m(out[k])
    if "pts" in out:
        out["pts"] = [m(p) for p in out["pts"]]
    for h in out.get("holes") or []:
        h["pts"] = [m(p) for p in h["pts"]]
    for q in out.get("parts") or []:
        if "pts" in q:
            q["pts"] = [m(p) for p in q["pts"]]
            for h in q.get("holes") or []:
                h["pts"] = [m(p) for p in h["pts"]]
        if "p" in q:
            q["p"] = m(q["p"])
    return out


def with_pts(e, *xy, **extra):
    out = json.loads(json.dumps(e))
    out["pts"] = pts(*xy)
    out.update(extra)
    return out


POINT_CASES = [
    ("Çizginin başı", LINE, place("start")), ("Çizginin sonu", LINE, place("end")),
    ("Çizginin ortası", LINE, place("mid", 0, 0, 0)),
    ("Alanın köşesi", ROOM, place("vertex", 0, 0, 2)),
    ("Alanın kapanan kenarının ortası", ROOM, place("mid", 0, 0, 3)),
    ("Açık yolun son kenarı yok", WALL, place("mid", 0, 0, 3)),
    ("Yaylı kenarın ortası", ARCED, place("mid", 0, 0, 0)),
    ("Deliğin köşesi", HOLED, place("vertex", 0, 1, 2)),
    ("İkinci parçanın köşesi", PARTS, place("vertex", 1, 0, 1)),
    ("Olmayan parça", PARTS, place("vertex", 2, 0, 0)),
    ("Dairenin merkezi", CIRCLE, place("center")),
    ("Dairenin başı yok", CIRCLE, place("start")),
    ("Yayın başı", ARC, place("start")), ("Yayın sonu", ARC, place("end")),
    ("Elipsin başı", ELLIPSE, place("start")), ("Elipsin sonu", ELLIPSE, place("end")),
    ("Çok noktalının ikinci noktası", POINTS, place("node", 2)),
]

FOLLOW_CASES = [
    ("Taşınan çizgi: baş izler", place("start"), LINE, moved(LINE, 1.5, 2)),
    ("Taşınan çizgi: orta izler", place("mid", 0, 0, 0), LINE, moved(LINE, 1.5, 2)),
    ("Silinen çizgi", place("end"), LINE, None),
    ("Çizgi yaya döndü", place("start"), LINE, {"kind": "arc", "c": v(2, 0), "r": 2, "a0": math.pi, "a1": 0.0}),
    ("Köşesi tutamaçla çekilen alan: köşe izler", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (5, 3.5), (0, 3))),
    ("Önüne köşe eklenen köşe yeni sırasına bağlanır", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4, 3), (0, 3))),
    ("Arkasına köşe eklenen köşe yerinde", place("vertex", 0, 0, 1), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (4, 3), (2, 3.5), (0, 3))),
    ("Silinen köşe", place("vertex", 0, 0, 2), ROOM, with_pts(ROOM, (0, 0), (4, 0), (0, 3))),
    ("Köşe hem taşındı hem köşe eklendi", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4.5, 3), (0, 3))),
    ("Kenarına köşe eklenen kenarın ortası kopar", place("mid", 0, 0, 1), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (4.5, 1.5), (4, 3), (0, 3))),
    ("Başka kenara köşe eklenince kenarın ortası yeni sırasında", place("mid", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4, 3), (0, 3))),
    ("Çoklu çizgi alana çevrildi: köşe bulunur", place("vertex", 0, 0, 3), WALL, dict(WALL, kind="polygon")),
    ("Çoklu çizgi alana çevrildi: kenar ortası", place("mid", 0, 0, 1), WALL, dict(WALL, kind="polygon")),
    ("Yayın bükümü değişti: kenar ortası izler", place("mid", 0, 0, 0), ARCED, dict(ARCED, bulges=[-0.25, 0.0])),
    ("Delik eklenen alanın dış köşesi yerinde", place("vertex", 0, 0, 2), ROOM,
     dict(ROOM, holes=[{"pts": pts((1, 1), (1, 2), (2, 2), (2, 1))}])),
    ("Deliği silinen alanın delik köşesi kopar", place("vertex", 0, 1, 0), HOLED, dict(HOLED, holes=None)),
    ("Parçaları ayrılan alan: ikinci parçanın köşesi bulunamaz", place("vertex", 1, 0, 2), PARTS,
     dict(PARTS, parts=None)),
    ("Parçası eklenen alanda ortak köşe iki kez: belirsiz", place("vertex", 0, 0, 1),
     with_pts(ROOM, (0, 0), (4, 0), (4, 3)),
     dict(with_pts(ROOM, (0, 0), (4, 0), (4, 3), (0, 3)), parts=[{"pts": pts((4, 0), (6, 0), (6, 2))}])),
    ("Taşınan dairenin merkezi", place("center"), CIRCLE, moved(CIRCLE, -1, 0.5)),
    ("Daire yaya kırıldı", place("center"), CIRCLE, {"kind": "arc", "c": v(1, 1), "r": 2, "a0": 0.0, "a1": 3.0}),
    ("Budanan yayın başı yeni başında", place("start"), ARC, dict(ARC, a0=math.pi / 6)),
    ("Elipsin sonu", place("end"), ELLIPSE, dict(ELLIPSE, t1=3.0)),
    ("Taşınan çok noktalı: düğüm izler", place("node", 1), POINTS, moved(POINTS, 1, 0)),
    ("Noktası silinen çok noktalı: düğüm yeni sırasında", place("node", 2), POINTS,
     dict(POINTS, parts=[{"p": v(3, 3)}])),
    ("Olmayan köşe", place("vertex", 0, 0, 9), ROOM, ROOM),
]

SNAP_CASES = [
    ("Uç nokta: çizginin sonu", LINE, "endpoint", v(4, 0)),
    ("Uç nokta: alanın köşesi", ROOM, "endpoint", v(4, 3)),
    ("Uç nokta: deliğin köşesi", HOLED, "endpoint", v(4, 2)),
    ("Uç nokta: yayın sonu", ARC, "endpoint", v(0.0, 2.0)),
    ("Orta nokta: kapanan kenar", ROOM, "midpoint", v(0, 1.5)),
    ("Orta nokta: çizgi", LINE, "midpoint", v(2, 0)),
    ("Orta nokta: yaylı kenar", ARCED, "midpoint", v(1, -0.5)),
    ("Merkez: daire", CIRCLE, "center", v(1, 1)),
    ("Nokta: çok noktalının ikincisi", POINTS, "node", v(3, 3)),
    ("Yakın: yer değil", LINE, "nearest", v(1, 0)),
    ("Uç nokta ama orada köşe yok", ROOM, "endpoint", v(2, 0)),
]

TRIG = {"Yayın başı", "Yayın sonu", "Elipsin başı", "Elipsin sonu", "Budanan yayın başı yeni başında", "Elipsin sonu"}


def build():
    points = []
    for name, e, pl in POINT_CASES:
        points.append({"name": name, "entity": e, "place": pl, "point": point(e, pl), "exact": name not in TRIG})
    follows = []
    for name, pl, before, after in FOLLOW_CASES:
        follows.append({"name": name, "place": pl, "before": before, "after": after,
                        "expect": follow(pl, before, after), "exact": name not in TRIG})
    snaps = [{"name": n, "entity": e, "snap": s, "at": at, "place": place_at(e, s, at)} for n, e, s, at in SNAP_CASES]
    return {
        "format": "kentos.subref-cases",
        "version": 1,
        "source": "scripts/fixtures/subref_cases.py (docs/adr/0249)",
        "points": points,
        "follows": follows,
        "snaps": snaps,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil", file=sys.stderr)
            return 1
        print(f"alt nesne başvuruları tutarlı: {len(build()['points'])} nokta, {len(build()['follows'])} izleme, {len(build()['snaps'])} kenet durumu")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
