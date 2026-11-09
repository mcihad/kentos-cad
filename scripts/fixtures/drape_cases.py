#!/usr/bin/env python3
"""The shared cases of `cad.entities.drape` (Yüzeyden kot ver; plan/map-cad.md MAP-06, docs/adr/4180), without
KentOS code.

    python3 scripts/fixtures/drape_cases.py           # writes fixtures/commands/v1/cad.entities.drape.json
    python3 scripts/fixtures/drape_cases.py --check   # writes nothing; compares

A Harita project (TUREF / TM30, from X0, Y0) whose terrain surface “Zemin” (t-1) is made of four points on `zemin`:
A (0, 0, 100), B (100, 0, 104), C (100, 80, 112), D (0, 100, 106). Its triangles are Delaunay's, decided here by the
empty circle: D lies outside the circle through A, B, C (and B outside A, C, D's), so the diagonal is AC and the
triangles ABC (z = 100 + 0.04 x + 0.1 y) and ACD (z = 100 + 0.072 x + 0.06 y). The rules, by hand (Civil 3D's
Elevations From Surface):

- each vertex of the objects (a point, a line's ends, every part's and hole's of a polyline or an area) inside a
  triangle (its edges too) takes that triangle's plane's height; one outside every triangle keeps its own;
- with `breaks` (Insert Intermediate Grade Break Points) a straight edge (bulge 0) of a polyline or an area without
  measures nor widths takes a vertex wherever it crosses a triangle's edge strictly between its ends (a crossing within
  1e-9 m of an end is that end), with the surface's height there, in order along the edge, its bulge 0; an arc is not
  split; a point, a line and a polyline with measures take none and are `unsplit`;
- the answer: each object's vertices set, outside and inserted, the totals and the surface's id; no vertex on the
  surface: `off_surface`.

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


def ring(pts, zs, bulges, closed, split):
    """A ring draped: (pts, zs, bulges, set, outside, inserted)."""
    n = len(pts)
    out_p, out_z, out_b = [], [], ([] if bulges is not None else None)
    s = o = ins = 0
    for k in range(n):
        x, y = local(pts[k])
        h = height(x, y)
        own = zs[k] if zs is not None else None
        if h is None:
            o += 1
            out_z.append(own)
        else:
            s += 1
            out_z.append(float(h))
        out_p.append(pts[k])
        bulge = bulges[k] if bulges is not None and k < len(bulges) else None
        if out_b is not None and bulge is not None:
            out_b.append(bulge)
        nxt = k + 1 if k + 1 < n else (0 if closed and n > 2 else None)
        if not split or nxt is None or (bulge or 0.0) != 0.0:
            continue
        qx, qy = local(pts[nxt])
        for t in crossings((x, y), (qx, qy)):
            ax, ay = x + t * (qx - x), y + t * (qy - y)
            h = height(ax, ay)
            out_p.append({"x": X0 + float(ax), "y": Y0 + float(ay)})
            out_z.append(float(h))
            if out_b is not None and bulge is not None:
                out_b.append(0.0)
            ins += 1
    return out_p, out_z, out_b, s, o, ins


def draped(e, split):
    """The object after and its row (without the uid)."""
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
            after["z"] = float(h)
            add(1, 0, 0)
    elif e["kind"] == "line":
        for key, zk in (("a", "za"), ("b", "zb")):
            h = height(*local(e[key]))
            if h is None:
                add(0, 1, 0)
            else:
                after[zk] = float(h)
                add(1, 0, 0)
    else:
        area = e["kind"] == "polygon"
        measured = any(m is not None for m in e.get("ms", [])) or "widths" in e
        brk = split and not measured

        def do(r, closed):
            pts, zs, bulges, s, o, i = ring(r["pts"], r.get("zs"), r.get("bulges"), closed, brk)
            r["pts"] = pts
            if any(z is not None for z in zs):
                r["zs"] = zs
            if bulges is not None:
                r["bulges"] = bulges
            add(s, o, i)
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
ENTS = SURFACE + [P1, P2, LINE, PL, POLY, ARC, MS, TEXT, LOCKED, FAR, KOTLU, CLOSED, PARTS]
SLOT = {name: i + 1 for i, name in enumerate(["A", "B", "C", "D", "P1", "P2", "LINE", "PL", "POLY", "ARC", "MS",
                                               "TEXT", "LOCKED", "FAR", "KOTLU", "CLOSED", "PARTS"])}
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


def ok(name, names, breaks=False, op="execute", surface="t-1", note=None):
    rows, fields = [], {}
    for n in names:
        e = ENTS[SLOT[n] - 1]
        after, row = draped(e, breaks)
        rows.append(dict({"uid": uid(SLOT[n])}, **row))
        fields[str(SLOT[n])] = {k: after[k] for k in KEYS if k in after}
    totals = {k: sum(r[k] for r in rows) for k in ("set", "outside", "inserted")}
    out = dict({"surface": "t-1", "objects": rows}, **totals, revision="$current")
    input_ = {"uids": [uid(SLOT[n]) for n in names], "surface": surface}
    if breaks:
        input_["breaks"] = True
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
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.drape", "commandVersion": 1,
           "title": "Yüzeyden kot ver", "note": "ADR 4180. Bağımsız başvuru scripts/fixtures/drape_cases.py: dört "
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
