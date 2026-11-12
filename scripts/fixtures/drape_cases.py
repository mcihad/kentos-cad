#!/usr/bin/env python3
"""The shared cases of `cad.entities.drape` (Yüzeyden kot ver; plan/map-cad.md MAP-06, docs/adr/4180, 4580 and 4980),
without KentOS code.

    python3 scripts/fixtures/drape_cases.py           # writes fixtures/commands/v1/cad.entities.drape.json
    python3 scripts/fixtures/drape_cases.py --check   # writes nothing; compares

A Harita project (TUREF / TM30, from X0, Y0) whose terrain surface “Zemin” (t-1) is made of four points on `zemin`:
A (0, 0, 100), B (100, 0, 104), C (100, 80, 112), D (0, 100, 106). Its triangles are Delaunay's, decided here by the
empty circle: D lies outside the circle through A, B, C (and B outside A, C, D's), so the diagonal is AC and the
triangles ABC (z = 100 + 0.04 x + 0.1 y) and ACD (z = 100 + 0.072 x + 0.06 y). The rules, by hand (ADR 4180):

- each vertex of the objects (a point, a line's ends, every part's and hole's of a polyline or an area) inside a
  triangle (its edges too) takes that triangle's plane's height; one outside every triangle keeps its own;
- with `breaks` (Insert Intermediate Grade Break Points) a straight edge (bulge 0) of a polyline or an area without
  measures nor widths takes a vertex wherever it crosses a triangle's edge strictly between its ends (a crossing within
  1e-9 m of an end is that end), with the surface's height there, in order along the edge, its bulge 0; an arc is not
  split; a point, a line and a polyline with measures take none and are `unsplit`;
- the answer: each object's vertices set, outside and inserted, the totals and the surface's id; no vertex on the
  surface: `off_surface`;
- ADR 4580 (the height over the surface and the range): with `offset` every height taken (vertices
  and break points) is the surface's plus it; with `range` (one object; a line's ends, a polyline's or an area's outer
  ring's vertices, 0-based, from < to) only the vertices from `from` to `to` take heights and are counted, only the edges
  between them (never the closing one) take break points, holes and further parts are left as they were;
- ADR 4980 (multi-part objects): the range's ring is `part` (0: the object's own vertices, 1 on: `parts` in order) and
  `ring` (0: that part's outer ring, 1 on: its holes in order), both 0 when absent; a hole is closed, a polyline's
  further part open, an area's closed; every other ring is left as it was. No such part or ring, or a line's part or
  ring not 0: `wrong_place` on `range.part` or `range.ring`; a range past the ring's vertices: `wrong_place`.

Heights and crossings are exact fractions in the project's own coordinates (from X0, Y0).
"""

import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.drape.json"
X0, Y0 = 500000.0, 4400000.0
STEP = "Yüzeyden kot ver"
SAME = {"revision": "same", "canUndo": False}
WITHIN = 1e-6


def uid(n):
    return f"0192f5c0-4180-7000-8000-{n:012x}"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


CORNERS = {"A": (0, 0, 100), "B": (100, 0, 104), "C": (100, 80, 112), "D": (0, 100, 106)}


def in_circle(a, b, c, d):
    """Whether d lies inside the circle through a, b, c (counter-clockwise), exactly."""
    m = [[F(p[0] - d[0]), F(p[1] - d[1])] for p in (a, b, c)]
    rows = [[x, y, x * x + y * y] for x, y in m]
    det = (rows[0][0] * (rows[1][1] * rows[2][2] - rows[1][2] * rows[2][1])
           - rows[0][1] * (rows[1][0] * rows[2][2] - rows[1][2] * rows[2][0])
           + rows[0][2] * (rows[1][0] * rows[2][1] - rows[1][1] * rows[2][0]))
    return det > 0


A, B, C, D = (CORNERS[k] for k in "ABCD")
# Delaunay: the diagonal whose triangles' circles hold no fourth corner.
assert not in_circle(A, B, C, D) and not in_circle(A, C, D, B), "the diagonal is AC"
TRIANGLES = [(A, B, C), (A, C, D)]
EDGES = [(A, B), (B, C), (C, A), (C, D), (D, A)]


def plane_z(t, x, y):
    """The height at (x, y) on the plane through triangle t's corners, exactly."""
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = [tuple(F(v) for v in p) for p in t]
    det = (x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1)
    a = ((z2 - z1) * (y3 - y1) - (z3 - z1) * (y2 - y1)) / det
    b = ((x2 - x1) * (z3 - z1) - (x3 - x1) * (z2 - z1)) / det
    return z1 + a * (x - x1) + b * (y - y1)


def holds(t, x, y):
    (x1, y1, _), (x2, y2, _), (x3, y3, _) = t

    def side(ax, ay, bx, by):
        return (F(bx) - F(ax)) * (y - F(ay)) - (F(by) - F(ay)) * (x - F(ax))
    s = [side(x1, y1, x2, y2), side(x2, y2, x3, y3), side(x3, y3, x1, y1)]
    return all(v >= 0 for v in s) or all(v <= 0 for v in s)


def height(x, y):
    """The surface's height at a place (exact, local), none off it."""
    for t in TRIANGLES:
        if holds(t, x, y):
            return plane_z(t, x, y)
    return None


def local(p):
    return F(str(round(p["x"] - X0, 9))), F(str(round(p["y"] - Y0, 9)))


def crossings(p, q):
    """Where segment pq crosses the triangles' edges strictly between its ends, sorted, once each."""
    (px, py), (qx, qy) = p, q
    rx, ry = qx - px, qy - py
    length2 = rx * rx + ry * ry
    out = []
    for c, d in EDGES:
        cx, cy, dx, dy = F(c[0]), F(c[1]), F(d[0]), F(d[1])
        sx, sy = dx - cx, dy - cy
        denom = rx * sy - ry * sx
        if denom == 0:
            continue
        wx, wy = cx - px, cy - py
        t = (wx * sy - wy * sx) / denom
        u = (wx * ry - wy * rx) / denom
        # Strictly inside: not within 1e-9 m of an end (exact here: the cases keep crossings far from the ends).
        if 0 <= u <= 1 and 0 < t < 1 and t * t * length2 > F(1, 10**18) and (1 - t) ** 2 * length2 > F(1, 10**18):
            out.append(t)
    return sorted(set(out))


def ring(pts, zs, bulges, closed, split, offset=F(0), rng=None):
    """A ring draped: (pts, zs, bulges, set, outside, inserted); `rng` the vertices taken (ADR 4580; none: all)."""
    n = len(pts)
    out_p, out_z, out_b = [], [], ([] if bulges is not None else None)
    s = o = ins = 0
    for k in range(n):
        x, y = local(pts[k])
        own = zs[k] if zs is not None else None
        taken = rng is None or rng[0] <= k <= rng[1]
        h = height(x, y) if taken else None
        if not taken:
            out_z.append(own)
        elif h is None:
            o += 1
            out_z.append(own)
        else:
            s += 1
            out_z.append(float(h + offset))
        out_p.append(pts[k])
        bulge = bulges[k] if bulges is not None and k < len(bulges) else None
        if out_b is not None and bulge is not None:
            out_b.append(bulge)
        nxt = k + 1 if k + 1 < n else (0 if closed and n > 2 else None)
        if not split or nxt is None or (bulge or 0.0) != 0.0:
            continue
        # With a range only the edges from its first vertex to its last take break points (never the closing one).
        if rng is not None and not (rng[0] <= k < rng[1]):
            continue
        qx, qy = local(pts[nxt])
        for t in crossings((x, y), (qx, qy)):
            ax, ay = x + t * (qx - x), y + t * (qy - y)
            h = height(ax, ay)
            out_p.append({"x": X0 + float(ax), "y": Y0 + float(ay)})
            out_z.append(float(h + offset))
            if out_b is not None and bulge is not None:
                out_b.append(0.0)
            ins += 1
    return out_p, out_z, out_b, s, o, ins


def ring_of(e, at):
    """ADR 4980: the ring `at` (part, ring) of a polyline or an area and whether it closes; None when there is none."""
    part, ring_ = at
    area = e["kind"] == "polygon"
    if part == 0:
        owner, closed = e, area or e.get("closed", False)
    else:
        parts = e.get("parts", [])
        if part > len(parts):
            return None
        owner, closed = parts[part - 1], area
    if ring_ == 0:
        return owner, closed
    holes = owner.get("holes", [])
    return (holes[ring_ - 1], True) if ring_ <= len(holes) else None


def draped(e, split, offset=F(0), rng=None, at=(0, 0)):
    """The object after and its row (without the uid); `offset` and `rng` as ADR 4580's, `at` ADR 4980's ring."""
    after = copy.deepcopy(e)
    row = {"set": 0, "outside": 0, "inserted": 0}

    def add(s, o, i):
        row["set"] += s
        row["outside"] += o
        row["inserted"] += i
    if e["kind"] == "point":
        h = height(*local(e["p"]))
        if h is None:
            add(0, 1, 0)
        else:
            after["z"] = float(h + offset)
            add(1, 0, 0)
    elif e["kind"] == "line":
        for k, (key, zk) in enumerate((("a", "za"), ("b", "zb"))):
            if rng is not None and not rng[0] <= k <= rng[1]:
                continue
            h = height(*local(e[key]))
            if h is None:
                add(0, 1, 0)
            else:
                after[zk] = float(h + offset)
                add(1, 0, 0)
    else:
        area = e["kind"] == "polygon"
        measured = any(m is not None for m in e.get("ms", [])) or "widths" in e
        brk = split and not measured

        def do(r, closed, within=None):
            pts, zs, bulges, s, o, i = ring(r["pts"], r.get("zs"), r.get("bulges"), closed, brk, offset, within)
            r["pts"] = pts
            if any(z is not None for z in zs):
                r["zs"] = zs
            if bulges is not None:
                r["bulges"] = bulges
            add(s, o, i)
        if rng is not None:
            # With a range only the ring it names takes heights (ADR 4980).
            r, closed = ring_of(after, at)
            do(r, closed, rng)
            if split and measured:
                row["unsplit"] = True
            return after, row
        do(after, area or e.get("closed", False))
        for h in after.get("holes", []):
            do(h, True)
        for part in after.get("parts", []):
            do(part, area)
            for h in part.get("holes", []):
                do(h, True)
        if split and measured:
            row["unsplit"] = True
    if split and e["kind"] in ("point", "line"):
        row["unsplit"] = True
    return after, row


SURFACE = [{"kind": "point", "layerId": "zemin", "attrs": {"Ad": k}, "p": P(v[0], v[1]), "z": float(v[2])}
           for k, v in CORNERS.items()]
P1 = {"kind": "point", "layerId": "cizim", "attrs": {"Ad": "R1"}, "p": P(60, 20)}
P2 = {"kind": "point", "layerId": "cizim", "attrs": {"Ad": "R2"}, "p": P(150, 50), "z": 99.0}
LINE = {"kind": "line", "layerId": "cizim", "attrs": {}, "a": P(20, 30), "b": P(130, 40), "zb": 98.5}
PL = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(10, 50), P(60, 30), P(90, 10)]}
POLY = {"kind": "polygon", "layerId": "cizim", "attrs": {}, "pts": [P(30, 10), P(70, 10), P(70, 60), P(30, 60)],
        "holes": [{"pts": [P(40, 30), P(52, 30), P(52, 38), P(40, 38)]}]}
ARC = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(10, 40), P(80, 40)], "bulges": [0.2, 0.0]}
MS = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(20, 70), P(80, 50)], "ms": [0.0, 63.2456]}
TEXT = {"kind": "text", "layerId": "cizim", "attrs": {}, "text": "Kot", "p": P(50, 50), "height": 2.0, "rotation": 0.0}
LOCKED = {"kind": "polyline", "layerId": "kilitli", "attrs": {}, "pts": [P(10, 10), P(20, 10)]}
FAR = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(200, 200), P(250, 220)]}
# Elevations already there are written over; a closed polyline's closing edge splits too; a second part as the first.
KOTLU = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(15, 20), P(45, 20)], "zs": [90.0, 91.0]}
CLOSED = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(20, 10), P(80, 10), P(80, 70)],
          "closed": True}
PARTS = {"kind": "polyline", "layerId": "cizim", "attrs": {}, "pts": [P(10, 90), P(40, 60)],
         "parts": [{"pts": [P(60, 20), P(70, 70)]}]}
# ADR 4980: a two-part area, each part with a hole; the second part and its hole cross the diagonal AC.
ADA = {"kind": "polygon", "layerId": "cizim", "attrs": {}, "pts": [P(5, 5), P(25, 5), P(25, 25), P(5, 25)],
       "holes": [{"pts": [P(10, 10), P(20, 10), P(20, 20), P(10, 20)]}],
       "parts": [{"pts": [P(50, 15), P(95, 15), P(95, 80), P(50, 80)],
                  "holes": [{"pts": [P(65, 35), P(85, 35), P(85, 65), P(65, 65)]}]}]}
ENTS = SURFACE + [P1, P2, LINE, PL, POLY, ARC, MS, TEXT, LOCKED, FAR, KOTLU, CLOSED, PARTS, ADA]
SLOT = {name: i + 1 for i, name in enumerate(["A", "B", "C", "D", "P1", "P2", "LINE", "PL", "POLY", "ARC", "MS",
                                               "TEXT", "LOCKED", "FAR", "KOTLU", "CLOSED", "PARTS", "ADA"])}
for i, e in enumerate(ENTS, 1):
    e["id"] = i


def setup(work="map", extra_surface=False):
    layers = [layer("0", "0"), layer("zemin", "Zemin noktaları"), layer("cizim", "Çizim"),
              layer("kilitli", "Kilitli", locked=True), layer("seyrek", "Seyrek")]
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work}
    if work in ("map", "road"):
        surfaces = [{"id": "t-1", "name": "Zemin", "points": {"layers": ["zemin"]}}]
        if extra_surface:
            surfaces.append({"id": "t-2", "name": "Seyrek", "points": {"layers": ["seyrek"]}})
        settings["road"] = {"surfaces": surfaces}
    return {"format": "kentos.document", "version": 2, "name": "Yüzeyden kot", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "cizim", "entities": copy.deepcopy(ENTS),
            "uids": [uid(i + 1) for i in range(len(ENTS))], "styles": {"items": [], "categories": []}}


KEYS = ["z", "za", "zb", "pts", "zs", "bulges", "holes", "parts", "ms"]


def ok(name, names, breaks=False, op="execute", surface="t-1", note=None, offset=None, rng=None, at=(0, 0)):
    rows, fields = [], {}
    for n in names:
        e = ENTS[SLOT[n] - 1]
        after, row = draped(e, breaks, F(str(offset)) if offset is not None else F(0), rng, at)
        rows.append(dict({"uid": uid(SLOT[n])}, **row))
        fields[str(SLOT[n])] = {k: after[k] for k in KEYS if k in after}
    totals = {k: sum(r[k] for r in rows) for k in ("set", "outside", "inserted")}
    out = dict({"surface": "t-1", "objects": rows}, **totals, revision="$current")
    input_ = {"uids": [uid(SLOT[n]) for n in names], "surface": surface}
    if breaks:
        input_["breaks"] = True
    if offset is not None:
        input_["offset"] = offset
    if rng is not None:
        input_["range"] = {"from": rng[0], "to": rng[1]}
        if at[0]:
            input_["range"]["part"] = at[0]
        if at[1]:
            input_["range"]["ring"] = at[1]
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    else:
        result = {"status": "completed", "output": out, "warnings": []}
    case = {"name": name}
    if note:
        case["note"] = note
    if op != "execute":
        case["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME, "within": WITHIN}]
        return case
    before = {str(SLOT[n]): {k: ENTS[SLOT[n] - 1][k] for k in KEYS if k in ENTS[SLOT[n] - 1]} for n in names}
    case["steps"] = [
        {"op": "execute", "input": input_, "result": result, "within": WITHIN,
         "expect": {"revision": "changed", "canUndo": True, "fields": fields}},
        {"op": "undo", "returns": STEP, "within": WITHIN, "expect": {"fields": before}}]
    return case


def refused(name, input_, code, message, path_, d=None):
    c = {"name": name}
    if d is not None:
        c["setup"] = d
    error = {"code": code, "message": message}
    if path_:
        # A refusal of the whole drawing (not a map or road project, a surface not made) names no field.
        error["path"] = path_
    c["steps"] = [{"op": "execute", "input": input_, "result": {"status": "failed", "error": error}, "expect": SAME}]
    return c


def cases():
    every = ["P1", "P2", "LINE", "PL", "POLY", "ARC", "MS"]
    sparse = setup(extra_surface=True)
    sparse["entities"].append({"kind": "point", "id": len(ENTS) + 1, "layerId": "seyrek", "attrs": {},
                               "p": P(0, 0), "z": 1.0})
    sparse["uids"].append(uid(len(ENTS) + 1))
    arch = setup(work="architecture")
    return [
        ok("Her köşe yüzeyin kotunu alır; yüzeyin dışındaki nokta ve çizgi ucu kendi kotunu korur; yay ve ölçülü çizgi "
           "de; geri alma", every,
           note="Köşegen AC: ABC'de z = 100 + 0.04x + 0.1y, ACD'de z = 100 + 0.072x + 0.06y (yerel koordinat)."),
        ok("Ara kırık noktalarıyla: köşegeni kesen düz kenarlara kotlu köşe eklenir (çoklu çizgiye 1, alanın dış "
           "halkasına 2, deliğine 2); yay bölünmez; nokta, çizgi ve ölçülü çizgi bölünmez", every, breaks=True),
        ok("Yüzey adıyla (büyük küçük harf ayrımı yok)", ["P1", "PL"], surface="zemin"),
        ok("Kotlu çoklu çizgi: eski kotları yüzeyinkiyle değişir", ["KOTLU"]),
        ok("Kapalı çoklu çizgi, ara kırık noktalarıyla: kapanış kenarına da köşe eklenir", ["CLOSED"], breaks=True),
        ok("İki parçalı çoklu çizgi, ara kırık noktalarıyla: ikinci parça da bölünür", ["PARTS"], breaks=True),
        ok("Yüzeyin dışındakiyle birlikte: yüzeydeki yazılır, dışındaki olduğu gibi kalır", ["P2", "FAR", "PL"]),
        ok("Plan yazmaz", ["P1", "POLY"], breaks=True, op="plan"),
        ok("Doğrulama", ["P1"], op="validate"),
        refused("Sürüm yazısı geçersiz", {"uids": [uid(SLOT["P1"])], "surface": "t-1", "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {
            "uids": [uid(SLOT["P1"])], "surface": "t-1", "expectedRevision": "7"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message":
                                            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                            "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": SAME}]},
        refused("Harita ya da Yol projesi değil", {"uids": [uid(SLOT["P1"])], "surface": "t-1"}, "not_a_road_project",
                "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil.",
                "", d=arch),
        refused("Projede olmayan yüzey", {"uids": [uid(SLOT["P1"])], "surface": "yok"}, "unknown_terrain",
                "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"),
        refused("Nesne yok", {"uids": [], "surface": "t-1"}, "invalid_input", "1 ile 10000 arasında nesne verin; 0 "
                "verildi.", "uids"),
        refused("Bir nesne iki kez", {"uids": [uid(SLOT["P1"]), uid(SLOT["P1"])], "surface": "t-1"}, "invalid_input",
                f"“{uid(SLOT['P1'])}” nesnesi iki kez verildi.", "uids[1]"),
        refused("Kimlik UUID değil", {"uids": ["x"], "surface": "t-1"}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uids[0]"),
        refused("Çizimde olmayan nesne", {"uids": [uid(99)], "surface": "t-1"}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uids[0]"),
        refused("Yazının köşesi yok", {"uids": [uid(SLOT["P1"]), uid(SLOT["TEXT"])], "surface": "t-1"}, "not_a_path",
                f"“{uid(SLOT['TEXT'])}” nokta, çizgi, çoklu çizgi ya da alan değil: köşesi yok, yüzeyden kot alamaz.",
                "uids[1]"),
        refused("Tek noktalı yüzey kurulamaz", {"uids": [uid(SLOT["P1"])], "surface": "t-2"}, "terrain_unbuildable",
                "$any", "", d=sparse),
        refused("Yüzeyin dışında: hiçbir köşe yüzeyde değil", {"uids": [uid(SLOT["FAR"])], "surface": "t-1"},
                "off_surface", "Nesnelerin hiçbir köşesi “Zemin” yüzeyinin üstünde değil; yüzeyin içindeki nesneleri "
                "seçin.", "uids"),
        refused("Kilitli katman", {"uids": [uid(SLOT["P1"]), uid(SLOT["LOCKED"])], "surface": "t-1"}, "layer_locked",
                "“Kilitli” katmanı kilitli: nesnenin kotları yazılamaz. Kilidi açın.", "uids[1]"),
    ] + relative_cases() + part_cases()


def relative_cases():
    """ADR 4580: the height over the surface and the range."""
    pl, poly, point = uid(SLOT["PL"]), uid(SLOT["POLY"]), uid(SLOT["P1"])
    off_nan = refused("Kot farkı sonlu değil", {"uids": [point], "surface": "t-1", "offset": 0.0}, "invalid_input",
                      "Yüzeyle kot farkı sonlu bir sayı olmalı (m).", "offset")
    off_nan["steps"][0]["nonFinite"] = {"offset": "NaN"}
    return [
        ok("Kot farkı 0,15 m: her köşe ve ara kırık noktası yüzeyin 0,15 m üstünde; dışındaki kendi kotunu korur",
           ["P1", "P2", "LINE", "PL", "POLY"], breaks=True, offset=0.15),
        ok("Kot farkı eksi: kotlu çoklu çizgi yüzeyin 0,5 m altına iner", ["KOTLU"], offset=-0.5),
        ok("Aralık 0–1: köşegeni kesen ilk kenar ara kırık noktası alır, üçüncü köşe kotsuz kalır", ["PL"], breaks=True,
           rng=(0, 1)),
        ok("Aralık 1–2: köşegeni kesen kenar aralığın dışında, bölünmez; ilk köşe kotsuz kalır", ["PL"], breaks=True,
           rng=(1, 2)),
        ok("Aralık 0–2, kot farkıyla: alanın dış halkasında üç köşe ve bir ara kırık noktası; kapanış kenarı ve delik "
           "olduğu gibi", ["POLY"], breaks=True, offset=0.15, rng=(0, 2)),
        ok("Aralık, iki parçalı çoklu çizgi: yalnız ilk parça kot alır", ["PARTS"], breaks=True, rng=(0, 1)),
        ok("Aralık plan yazmaz", ["POLY"], breaks=True, rng=(0, 2), op="plan"),
        off_nan,
        refused("Aralık iki nesneye verilmez", {"uids": [pl, poly], "surface": "t-1", "range": {"from": 0, "to": 1}},
                "invalid_input", "Köşe aralığı tek bir nesneye verilir; 2 nesne verildi.", "range"),
        refused("Noktaya aralık verilmez", {"uids": [point], "surface": "t-1", "range": {"from": 0, "to": 1}},
                "not_a_path", "Nokta tek köşelidir: köşe aralığı çizgi, çoklu çizgi ya da alanın köşelerine verilir.",
                "range"),
        refused("Aralığın başı sonundan önce değil", {"uids": [pl], "surface": "t-1", "range": {"from": 2, "to": 1}},
                "wrong_place", "Köşe aralığı 2–1 olamaz: nesnenin 3 köşesi var (0'dan), başlangıç bitişten önce olmalı.",
                "range"),
        refused("Aralık köşelerin dışında", {"uids": [pl], "surface": "t-1", "range": {"from": 1, "to": 3}},
                "wrong_place", "Köşe aralığı 1–3 olamaz: nesnenin 3 köşesi var (0'dan), başlangıç bitişten önce olmalı.",
                "range"),
        refused("Aralığın hiçbir köşesi yüzeyde değil", {"uids": [uid(SLOT["FAR"])], "surface": "t-1",
                                                           "range": {"from": 0, "to": 1}}, "off_surface",
                "Köşe aralığının (0–1) hiçbir köşesi “Zemin” yüzeyinin üstünde değil; aralığı yüzeyin içinde seçin.",
                "uids"),
    ]


def part_cases():
    """ADR 4980: the range on a multi-part object's further part or on a hole."""
    parts, ada, line = uid(SLOT["PARTS"]), uid(SLOT["ADA"]), uid(SLOT["LINE"])
    return [
        ok("Aralık, iki parçalı çoklu çizginin ikinci parçası: yalnız o kot alır, köşegeni kestiği yerde ara kırık noktası",
           ["PARTS"], breaks=True, rng=(0, 1), at=(1, 0)),
        ok("Aralık, iki parçalı alanın ikinci parçası 0–2: köşegeni kesen kenar bölünür; kapanış kenarı, delikler ve ilk "
           "parça olduğu gibi", ["ADA"], breaks=True, rng=(0, 2), at=(1, 0)),
        ok("Aralık, ikinci parçanın deliği 1–3, kot farkıyla: köşegeni kesen kenar bölünür, ilk köşe olduğu gibi",
           ["ADA"], breaks=True, offset=0.15, rng=(1, 3), at=(1, 1)),
        ok("Aralık, ilk parçanın deliği 0–2", ["ADA"], rng=(0, 2), at=(0, 1)),
        ok("Delikteki aralık plan yazmaz", ["ADA"], breaks=True, rng=(1, 3), at=(1, 1), op="plan"),
        refused("Olmayan parça", {"uids": [parts], "surface": "t-1", "range": {"from": 0, "to": 1, "part": 2}},
                "wrong_place", "Nesnenin 2 parçası var (0'dan): 2. parça yok.", "range.part"),
        refused("Olmayan delik", {"uids": [ada], "surface": "t-1", "range": {"from": 0, "to": 1, "ring": 2}},
                "wrong_place", "0. parçanın 1 deliği var: 2. halka yok (0 dış halka, 1'den delikler).", "range.ring"),
        refused("Çizginin ikinci parçası yok", {"uids": [line], "surface": "t-1", "range": {"from": 0, "to": 1, "part": 1}},
                "wrong_place", "Çizginin tek parçası ve tek halkası var: parça ve halka 0 olmalı.", "range.part"),
        refused("Delikte aralık köşelerin dışında", {"uids": [ada], "surface": "t-1",
                                                    "range": {"from": 2, "to": 4, "part": 1, "ring": 1}},
                "wrong_place", "Köşe aralığı 2–4 olamaz: 1. parçanın 1. halkasının 4 köşesi var (0'dan), başlangıç "
                "bitişten önce olmalı.", "range"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.drape", "commandVersion": 1,
           "title": "Yüzeyden kot ver", "note": "ADR 4180, 4580 ve 4980. Bağımsız başvuru scripts/fixtures/drape_cases.py: dört "
           "noktanın Delaunay üçgenleri boş çember sınamasıyla, düzlem kotları ve kenar kesişimleri kesirlerle; "
           "KentOS kodu yok.", "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Yüzeyden kot ver ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
