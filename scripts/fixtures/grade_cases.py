#!/usr/bin/env python3
"""The shared cases of `cad.entities.grade` (Eğimle kot ver; plan/map-cad.md MAP-06, docs/adr/1290, 3380 and
4980), without KentOS code.

    python3 scripts/fixtures/grade_cases.py           # writes fixtures/commands/v1/cad.entities.grade.json
    python3 scripts/fixtures/grade_cases.py --check   # writes nothing; compares

The rule, by hand: the horizontal length along the path from `from` to each vertex up to `to` (a straight edge its
chord, an arc edge with bulge b its arc: θ = 4·atan|b|, r = chord / (2·sin(θ/2)), length r·θ); the grade g = (end −
start) / L or the one given; each vertex start + g·s, the last the end as given (or start + g·L); the start is the one
given, else the vertex's own elevation. Vertices outside the range keep theirs.

The high or low point (docs/adr/3380), by hand: the line from the start
rising `ahead` and the line from the end rising `back` towards the start meet at s = (end − start + back·L) / (ahead +
back); none when the grades are parallel or s is not strictly inside (0, L). On the edge holding s at the fraction t of
its length: t within 10⁻⁹ of an end is that vertex (none when it is the range's start or end), else a new vertex there:
on a straight edge the chord's point, on an arc the start's radius turned by t of the arc's angle about the centre (the
centre on the chord's left normal at r·cos(θ/2), θ = 4·atan(b) signed), the halves' bulges tan(t·θ/4) and
tan((1 − t)·θ/4). Vertices up to s take start + ahead·d, after it end + back·(L − d); the point start + ahead·s.

Docs/adr/4980: the range lies on the ring `ring` (0 the outer one, 1 on the holes) of the part `part` (0 the object's own
vertices, 1 on `parts` in order), both 0 when absent; the rules above on that ring alone, every other ring left. No such
part or ring: `wrong_place` on `part` or `ring`; a range past the ring's vertices: `wrong_place` on `to`, naming the
ring.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.grade.json"
X0, Y0 = 500000.0, 4400000.0


def uid(n):
    return f"0192f5c0-1290-7000-8000-{n:012x}"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


POLY = {"kind": "polyline", "id": 1, "layerId": "tasarim", "attrs": {}, "pts": [P(0, 0), P(30, 0), P(30, 40), P(80, 40),
                                                                                 P(100, 40)],
        "bulges": [0.0, 0.0, 0.5, 0.0, 0.0], "zs": [850.0, None, None, 851.25, None]}
LINE = {"kind": "line", "id": 2, "layerId": "tasarim", "attrs": {}, "a": P(0, 100), "b": P(60, 180), "za": 900.0}
AREA = {"kind": "polygon", "id": 3, "layerId": "tasarim", "attrs": {}, "pts": [P(200, 0), P(240, 0), P(240, 30),
                                                                               P(200, 30)]}
LOCKED = {"kind": "polyline", "id": 4, "layerId": "kilitli", "attrs": {}, "pts": [P(0, 300), P(10, 300)],
          "zs": [10.0, None]}
TEXT = {"kind": "text", "id": 5, "layerId": "tasarim", "attrs": {}, "text": "Kot", "p": P(0, 400), "height": 2.0, "rotation": 0.0}
# Docs/adr/4980: a two-part area, each part with a hole; a two-part polyline whose second part has elevations.
ISLAND = {"kind": "polygon", "id": 6, "layerId": "tasarim", "attrs": {}, "pts": [P(300, 0), P(340, 0), P(340, 30),
                                                                                P(300, 30)],
          "holes": [{"pts": [P(310, 10), P(330, 10), P(330, 20), P(310, 20)]}],
          "parts": [{"pts": [P(400, 0), P(440, 0), P(440, 30), P(400, 30)],
                     "holes": [{"pts": [P(410, 10), P(430, 10), P(430, 20), P(410, 20)]}]}]}
TWO = {"kind": "polyline", "id": 7, "layerId": "tasarim", "attrs": {}, "pts": [P(0, 500), P(40, 500)],
       "parts": [{"pts": [P(0, 520), P(40, 520), P(80, 540)], "bulges": [0.0, 0.25, 0.0], "zs": [700.0, None, 702.0]}]}
ENTS = [POLY, LINE, AREA, LOCKED, TEXT, ISLAND, TWO]


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Eğimle kot",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": [layer("0", "0"), layer("tasarim", "Tasarım"),
                                                     layer("kilitli", "Kilitli", locked=True)],
            "activeLayer": "tasarim", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


def edge(a, b, bulge):
    c = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
    if bulge == 0.0:
        return c
    t = 4.0 * math.atan(abs(bulge))
    return c / (2.0 * math.sin(t / 2.0)) * t


def ring_of(e, at):
    """The ring `at` (part, ring) of a polyline or an area (docs/adr/4980): the dict holding its pts, bulges, zs."""
    part, ring = at
    owner = e if part == 0 else e["parts"][part - 1]
    return owner if ring == 0 else owner["holes"][ring - 1]


def path(e, at=(0, 0)):
    if e["kind"] == "line":
        return [e["a"], e["b"]], [0.0], [e.get("za"), e.get("zb")]
    r = ring_of(e, at)
    n = len(r["pts"])
    return r["pts"], r.get("bulges", [0.0] * n), r.get("zs", [None] * n)


def graded(e, frm, to, start=None, end=None, grade=None, at=(0, 0)):
    pts, bulges, zs = path(e, at)
    s0 = start if start is not None else zs[frm]
    along = [0.0]
    for i in range(frm, to):
        along.append(along[-1] + edge(pts[i], pts[(i + 1) % len(pts)], bulges[i] if i < len(bulges) else 0.0))
    L = along[-1]
    g = (end - s0) / L if end is not None else grade
    out = [s0 + g * d for d in along]
    last = end if end is not None else out[-1]
    out[-1] = last
    return {"length": L, "start": s0, "end": last, "grade": g, "zs": out}


SAME = {"revision": "same", "canUndo": False}


def turned(c, a, angle):
    """`a` turned about `c` by `angle` (counter-clockwise positive)."""
    dx, dy = a["x"] - c["x"], a["y"] - c["y"]
    ca, sa = math.cos(angle), math.sin(angle)
    return {"x": c["x"] + dx * ca - dy * sa, "y": c["y"] + dx * sa + dy * ca}


def arc_centre(a, b, bulge):
    """The centre of the arc from `a` to `b` of `bulge`: on the chord's left normal (right for a clockwise arc)."""
    c = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
    theta = 4.0 * math.atan(bulge)
    r = c / (2.0 * math.sin(abs(theta) / 2.0))
    d = r * math.cos(theta / 2.0) * (1.0 if theta > 0 else -1.0)
    nx, ny = -(b["y"] - a["y"]) / c, (b["x"] - a["x"]) / c
    return {"x": (a["x"] + b["x"]) / 2.0 + nx * d, "y": (a["y"] + b["y"]) / 2.0 + ny * d}, theta


class NoPoint(Exception):
    def __init__(self, why, along=None, length=None):
        super().__init__(why)
        self.why, self.along, self.length = why, along, length


def peaked(e, frm, to, start=None, end=None, ahead=None, back=None, at=(0, 0)):
    """The object through its high or low point and the command's answer, by the rule above."""
    pts, bulges, zs = path(e, at)
    s0 = start if start is not None else zs[frm]
    z1 = end if end is not None else zs[to]
    lens = [edge(pts[i], pts[i + 1], bulges[i] if i < len(bulges) else 0.0) for i in range(frm, to)]
    along = [0.0]
    for x in lens:
        along.append(along[-1] + x)
    L = along[-1]
    if ahead + back == 0.0:
        raise NoPoint("parallel")
    s = (z1 - s0 + back * L) / (ahead + back)
    if not (0.0 < s < L):
        raise NoPoint("outside", s, L)
    k = next(k for k in range(len(lens)) if lens[k] > 0.0 and s <= along[k + 1])
    t = min(max((s - along[k]) / lens[k], 0.0), 1.0)
    i = frm + k
    if t <= 1e-9 or t >= 1.0 - 1e-9:
        v = k if t <= 1e-9 else k + 1
        if v in (0, len(lens)):
            raise NoPoint("outside", s, L)
        at_vertex, d_at, p = frm + v, along[v], pts[frm + v]
    else:
        at_vertex, d_at = None, s
        b = bulges[i] if i < len(bulges) else 0.0
        if b == 0.0:
            a_, b_ = pts[i], pts[i + 1]
            p = {"x": a_["x"] + (b_["x"] - a_["x"]) * t, "y": a_["y"] + (b_["y"] - a_["y"]) * t}
            halves = [0.0, 0.0]
        else:
            c, theta = arc_centre(pts[i], pts[i + 1], b)
            p = turned(c, pts[i], theta * t)
            halves = [math.tan(theta * t / 4.0), math.tan(theta * (1.0 - t) / 4.0)]
    z = s0 + ahead * d_at
    new = []
    for v, d in enumerate(along):
        if v == 0:
            new.append(s0)
        elif v == len(along) - 1:
            new.append(z1)
        elif at_vertex == frm + v:
            new.append(z)
        elif d <= d_at:
            new.append(s0 + ahead * d)
        else:
            new.append(z1 + back * (L - d))
    kind = "high" if ahead > 0 and back > 0 else "low" if ahead < 0 and back < 0 else "break"
    after = copy.deepcopy(e)
    ring = ring_of(after, at) if e["kind"] != "line" else None
    all_zs = list(zs) if e["kind"] != "line" else [e.get("za"), e.get("zb")]
    for j, v in enumerate(new):
        all_zs[frm + j] = v
    if at_vertex is not None:
        index, inserted, out_zs = at_vertex, False, new
        ring["zs"] = all_zs
    else:
        index, inserted = i + 1, True
        out_zs = new[:k + 1] + [z] + new[k + 1:]
        all_zs.insert(i + 1, z)
        if e["kind"] == "line":
            after = {"kind": "polyline", "id": e["id"], "layerId": e["layerId"], "attrs": e["attrs"],
                     "pts": [e["a"], p, e["b"]], "zs": all_zs}
        else:
            ring["pts"] = pts[:i + 1] + [p] + pts[i + 1:]
            if "bulges" in ring:
                bs = list(bulges)
                ring["bulges"] = bs[:i] + halves + bs[i + 1:]
            ring["zs"] = all_zs
    point = {"index": index, "inserted": inserted, "along": d_at, "x": p["x"], "y": p["y"], "z": z, "kind": kind}
    out = {"length": L, "start": s0, "end": z1, "grade": ahead, "zs": out_zs, "back": back, "point": point}
    return after, out


def where(at):
    """The input's part and ring (docs/adr/4980), absent when 0."""
    return {k: v for k, v in (("part", at[0]), ("ring", at[1])) if v}


def peak_ok(name, slot, frm, to, op="execute", at=(0, 0), **kw):
    """A high or low point case; coordinates within 10⁻⁶ m (two computations of a point near 4 400 000 m)."""
    e = ENTS[slot - 1]
    after, out = peaked(e, frm, to, at=at, **kw)
    input_ = {"uid": uid(slot), "from": frm, "to": to, **where(at), **{k: v for k, v in kw.items() if v is not None}}
    result = {"status": "completed", "output": dict(out, revision="$current"), "warnings": []}
    if op != "execute":
        return {"name": name, "steps": [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-6}]}
    keys = ["kind", "pts", "bulges", "zs", "a", "b", "za", "zb", "holes", "parts"]
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": result, "within": 1e-6,
         "expect": {"revision": "changed", "canUndo": True, "fields": {str(slot): {k: after.get(k) for k in keys}}}},
        {"op": "undo", "returns": "Eğimle kot ver", "within": 1e-6,
         "expect": {"fields": {str(slot): {k: e.get(k) for k in keys}}}}]}


def outside_message(slot, frm, to, **kw):
    try:
        peaked(ENTS[slot - 1], frm, to, **kw)
    except NoPoint as n:
        assert n.why == "outside", n.why
        # Three decimals as the message writes them: not at a half, where two roundings could part.
        for v in (n.along, n.length):
            assert abs(v * 1000 - math.floor(v * 1000) - 0.5) > 1e-6, v
        return (f"İki eğim aralığın içinde kesişmiyor: kesişme yol boyunca başlangıçtan {n.along:.3f} m'de, aralık 0 "
                f"ile {n.length:.3f} m arası (uçlar sayılmaz). Eğimleri ya da kotları değiştirin.")
    raise AssertionError("a point")


def ok(name, slot, frm, to, op="execute", at=(0, 0), **kw):
    e = ENTS[slot - 1]
    g = graded(e, frm, to, at=at, **kw)
    input_ = {"uid": uid(slot), "from": frm, "to": to, **where(at), **{k: v for k, v in kw.items() if v is not None}}
    out = dict(g, revision="$current")
    if op == "validate":
        result = {"status": "completed", "output": None, "warnings": []}
    else:
        result = {"status": "completed", "output": out, "warnings": []}
    if op != "execute":
        return {"name": name, "steps": [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-9}]}
    after = copy.deepcopy(e)
    if e["kind"] == "line":
        after["za"], after["zb"] = g["zs"][0], g["zs"][1]
        fields = {"za": after["za"], "zb": after["zb"]}
    else:
        ring = ring_of(after, at)
        zs = list(ring.get("zs", [None] * len(ring["pts"])))
        for k, v in enumerate(g["zs"]):
            zs[frm + k] = v
        ring["zs"] = zs
        # The ring's own field, or the whole holes or parts it is in (docs/adr/4980).
        fields = {"zs": zs} if at == (0, 0) else {k: after.get(k) for k in ("zs", "holes", "parts")}
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": result, "within": 1e-9,
         "expect": {"revision": "changed", "canUndo": True, "fields": {str(slot): fields}}},
        {"op": "undo", "returns": "Eğimle kot ver", "expect": {"fields": {str(slot): {k: e.get(k) for k in fields}}}}]}


def refused(name, input_, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    return [
        ok("Eğimle: kendi kotundan %2, düz ve yaylı kenar boyunca; geri alma", 1, 0, 4, grade=0.02),
        ok("Bitiş kotuyla: eğim ondan çıkar", 1, 0, 3, end=852.0),
        ok("Ara köşeden başlar; aralık dışı köşeler kotlarını korur", 1, 1, 3, start=849.5, grade=-0.015),
        ok("Yaylı kenarın boyu yayı boyuncadır", 1, 2, 3, start=100.0, end=101.0),
        ok("Eksi eğim", 1, 0, 2, grade=-0.05),
        ok("Çizginin iki ucu", 2, 0, 1, grade=0.03),
        ok("Çizgi: bitiş kotuyla", 2, 0, 1, end=905.5),
        ok("Alanın dış halkası (kotsuz): başlangıç verilir", 3, 0, 3, start=120.0, grade=0.01),
        ok("Plan yazmaz", 1, 0, 4, op="plan", grade=0.02),
        ok("Doğrulama", 1, 0, 4, op="validate", grade=0.02),
        refused("Kotsuz köşeden başlangıç verilmeden", {"uid": uid(3), "from": 0, "to": 2, "grade": 0.01},
                "elevation_missing", "0. köşenin kotu yok; başlangıç kotunu (start) verin.", "start"),
        refused("Hem bitiş hem eğim", {"uid": uid(1), "from": 0, "to": 2, "end": 851.0, "grade": 0.01}, "invalid_input",
                "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.", "end"),
        refused("Ne bitiş ne eğim", {"uid": uid(1), "from": 0, "to": 2}, "invalid_input",
                "Bitiş kotunu (end) ya da eğimi (grade) verin, ikisini birden değil.", "end"),
        refused("Ters aralık", {"uid": uid(1), "from": 3, "to": 1, "grade": 0.01}, "wrong_place",
                "Köşe aralığı 3–1 olamaz: nesnenin 5 köşesi var (0'dan), başlangıç bitişten önce olmalı.", "to"),
        refused("Köşe sayısının dışında", {"uid": uid(1), "from": 0, "to": 5, "grade": 0.01}, "wrong_place",
                "Köşe aralığı 0–5 olamaz: nesnenin 5 köşesi var (0'dan), başlangıç bitişten önce olmalı.", "to"),
        refused("Yazı bir yol değil", {"uid": uid(5), "from": 0, "to": 1, "grade": 0.01}, "not_a_path",
                "Nesne çizgi, çoklu çizgi ya da alan değil; kotu eğimle verilemez.", "uid"),
        refused("Kilitli katman", {"uid": uid(4), "from": 0, "to": 1, "grade": 0.01}, "layer_locked",
                "“Kilitli” katmanı kilitli: nesnenin kotları değişemez. Kilidi açın.", "uid"),
        refused("Çizimde olmayan nesne", {"uid": uid(99), "from": 0, "to": 1, "grade": 0.01}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok.", "uid"),
        refused("Kimlik UUID değil", {"uid": "x", "from": 0, "to": 1, "grade": 0.01}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil.", "uid"),
        refused("Sürüm yazısı geçersiz", {"uid": uid(1), "from": 0, "to": 1, "grade": 0.01, "expectedRevision": "abc"},
                "invalid_revision", "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
        # Yüksek/alçak nokta (ADR 3380).
        peak_ok("Yüksek nokta düz kenarda: yeni köşe, kendi kotlarından; geri alma", 1, 0, 3, ahead=0.04, back=0.01),
        peak_ok("Yüksek nokta yaylı kenarda: iki yarı aynı çemberde", 1, 0, 3, ahead=0.03, back=0.02),
        peak_ok("Alçak nokta: plan yazmaz", 1, 0, 3, op="plan", ahead=-0.02, back=-0.03),
        peak_ok("Ne yüksek ne alçak: eğim değişir", 1, 0, 3, ahead=0.02, back=-0.005),
        peak_ok("Kesişme ara köşeye düşer: köşe eklenmez, ona kot verilir", 1, 0, 2, end=850.625, ahead=0.0625,
                back=0.03125),
        peak_ok("Çizgi Köşe ekle'deki gibi çoklu çizgi olur", 2, 0, 1, end=899.0, ahead=0.02, back=0.03),
        peak_ok("Alanın dış halkası: başlangıç ve bitiş verilir", 3, 0, 3, start=120.0, end=119.5, ahead=0.01, back=0.015),
        refused("Eğim ile yüksek/alçak nokta birlikte", {"uid": uid(1), "from": 0, "to": 3, "grade": 0.02, "ahead": 0.03,
                                                         "back": 0.02}, "invalid_input",
                "Eğim (grade) ile başlangıçtan ve bitişten eğimler (ahead, back) birlikte verilmez: ya sabit eğim ya "
                "yüksek/alçak nokta.", "grade"),
        refused("Yalnız başlangıçtan eğim", {"uid": uid(1), "from": 0, "to": 3, "ahead": 0.03}, "invalid_input",
                "Yüksek/alçak nokta iki eğimle bulunur: başlangıçtan eğimi (ahead) ve bitişten eğimi (back) birlikte verin.",
                "back"),
        refused("Yalnız bitişten eğim", {"uid": uid(1), "from": 0, "to": 3, "back": 0.02}, "invalid_input",
                "Yüksek/alçak nokta iki eğimle bulunur: başlangıçtan eğimi (ahead) ve bitişten eğimi (back) birlikte verin.",
                "ahead"),
        refused("Bitiş köşesinin kotu yok", {"uid": uid(1), "from": 0, "to": 2, "ahead": 0.03, "back": 0.02},
                "elevation_missing", "2. köşenin kotu yok; bitiş kotunu (end) verin.", "end"),
        refused("Paralel eğimler kesişmez", {"uid": uid(1), "from": 0, "to": 3, "ahead": 0.02, "back": -0.02},
                "no_intersection", "Başlangıçtan ve bitişten eğimler paralel (biri öbürünün eksisi): hiçbir yerde "
                "kesişmezler. Eğimlerden birini değiştirin.", "back"),
        refused("Kesişme aralığın dışında", {"uid": uid(1), "from": 0, "to": 3, "ahead": 0.05, "back": -0.03},
                "no_intersection", outside_message(1, 0, 3, ahead=0.05, back=-0.03), "back"),
        refused("Kesişme bitişte: uçlar sayılmaz", {"uid": uid(1), "from": 0, "to": 2, "end": 854.375, "ahead": 0.0625,
                                                    "back": 0.03125},
                "no_intersection", outside_message(1, 0, 2, end=854.375, ahead=0.0625, back=0.03125), "back"),
        # Çok parçalı nesne (ADR 4980).
        ok("İkinci parçanın dış halkası, eğimle: öbür halkalar olduğu gibi", 6, 0, 2, start=100.0, grade=0.02,
           at=(1, 0)),
        ok("İkinci parçanın deliği, bitiş kotuyla", 6, 1, 3, start=101.0, end=101.6, at=(1, 1)),
        ok("İlk parçanın deliği: plan yazmaz", 6, 0, 3, op="plan", start=99.0, grade=-0.01, at=(0, 1)),
        peak_ok("Çoklu çizginin ikinci parçasında yüksek nokta: yaylı kenara köşe eklenir, ilk parça olduğu gibi", 7, 0, 2,
                ahead=0.05, back=0.03, at=(1, 0)),
        refused("Olmayan parça", {"uid": uid(6), "from": 0, "to": 1, "part": 2, "start": 100.0, "grade": 0.01},
                "wrong_place", "Nesnenin 2 parçası var (0'dan): 2. parça yok.", "part"),
        refused("Olmayan delik", {"uid": uid(6), "from": 0, "to": 1, "part": 1, "ring": 2, "start": 100.0,
                                  "grade": 0.01},
                "wrong_place", "1. parçanın 1 deliği var: 2. halka yok (0 dış halka, 1'den delikler).", "ring"),
        refused("Çizginin ikinci halkası yok", {"uid": uid(2), "from": 0, "to": 1, "ring": 1, "grade": 0.01},
                "wrong_place", "Çizginin tek parçası ve tek halkası var: parça ve halka 0 olmalı.", "ring"),
        refused("Delikte aralık köşelerin dışında", {"uid": uid(6), "from": 2, "to": 4, "part": 1, "ring": 1,
                                                    "start": 100.0, "grade": 0.01},
                "wrong_place", "Köşe aralığı 2–4 olamaz: 1. parçanın 1. halkasının 4 köşesi var (0'dan), başlangıç "
                "bitişten önce olmalı.", "to"),
        {"name": "Beklenen sürüm eski: çatışma", "steps": [{"op": "execute", "input": {
            "uid": uid(1), "from": 0, "to": 1, "grade": 0.01, "expectedRevision": "7"}, "result": {
            "status": "conflict", "error": {"code": "revision_conflict", "message":
                                            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                            "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                                            "revision": "$current"}}, "expect": SAME}]},
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.grade", "commandVersion": 1,
           "title": "Eğimle kot ver", "note": "ADR 1290, 3380, 4980. Bağımsız başvuru scripts/fixtures/grade_cases.py: yay "
           "boyu, kotlar, yüksek/alçak noktanın yeri ve yayın yarıları elle; KentOS kodu yok.", "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Eğimle kot ver ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
